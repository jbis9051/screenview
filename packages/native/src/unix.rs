use crate::api::{self, *};
use errno::{errno, Errno};
use image::RgbImage;
use libc::{c_int, c_void, shmat, shmctl, shmdt, shmget, size_t, IPC_CREAT, IPC_PRIVATE, IPC_RMID};
use neon::prelude::Finalize;
use x11_clipboard::{Clipboard, error::Error as X11ClipboardError};
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Formatter},
    ptr,
    str, time::Duration,
};
use x11::{
    xlib::{XDefaultRootWindow, XKeysymToKeycode, XOpenDisplay, XSync},
    xtest::{XTestFakeButtonEvent, XTestFakeKeyEvent},
};
use xcb::{
    randr::GetMonitors,
    shm::{Attach, Detach, GetImage, Seg},
    x::{
        Drawable,
        GetAtomName,
        GetGeometry,
        GetProperty,
        GetWindowAttributes,
        MapState,
        QueryPointer,
        QueryTree,
        WarpPointer,
        Window,
        ATOM_STRING,
        ATOM_WM_NAME,
    },
    ConnError,
    Connection,
    ProtocolError,
    Xid,
    XidNew,
};

struct X11MonitorInfo {
    name: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl MouseButton {
    #[inline(always)]
    fn id(&self) -> u32 {
        match self {
            MouseButton::Left => 1,
            MouseButton::Center => 2,
            MouseButton::Right => 3,
            MouseButton::ScrollUp => 4,
            MouseButton::ScrollDown => 5,
            MouseButton::ScrollLeft => 6,
            MouseButton::ScrollRight => 7,
            MouseButton::Button4 => 8,
            MouseButton::Button5 => 9,
        }
    }
}

pub struct X11Api {
    // General fields
    conn: Connection,
    root: Window,

    // Screen capture fields
    width: u16,
    height: u16,
    depth: usize,
    shmid: c_int,
    shmaddr: *mut u32,
    shmseg: Seg,

    // Monitor map
    monitors: Vec<X11MonitorInfo>,

    // Clipboard API
    clipboard: Clipboard
}

unsafe impl Send for X11Api {}

impl NativeApiTemplate for X11Api {
    type Error = Error;

    fn new() -> Result<Self, Self::Error> {
        let dpy = unsafe { XOpenDisplay(ptr::null()) };
        if dpy.is_null() {
            return Err(Error::DisplayOpenFailed);
        }
        let root = unsafe { Window::new(XDefaultRootWindow(dpy) as u32) };
        let conn = unsafe { Connection::from_xlib_display(dpy) };

        let cookie = conn.send_request(&GetGeometry {
            drawable: Drawable::Window(root),
        });
        let reply = conn.wait_for_reply(cookie)?;

        let width = reply.width();
        let height = reply.height();
        let depth = reply.depth() as size_t;

        let (shmid, shmaddr, shmseg) =
            Self::init_shm(&conn, width as size_t * height as size_t * depth)?;

        Ok(Self {
            conn,
            root,
            width,
            height,
            depth,
            shmid,
            shmaddr,
            shmseg,
            monitors: Vec::new(),
            clipboard: Clipboard::new()?
        })
    }

    fn key_toggle(&self, keysym: u32, down: bool) {
        let dpy = self.conn.get_raw_dpy();
        let down = if down { 1 } else { 0 };

        unsafe {
            let keycode = XKeysymToKeycode(dpy, keysym as _);
            XTestFakeKeyEvent(dpy, keycode as _, down, 0);
            XSync(dpy, 0);
        }
    }

    fn pointer_position(&self) -> Result<MousePosition, Self::Error> {
        let reply = self
            .conn
            .wait_for_reply(self.conn.send_request(&QueryPointer { window: self.root }))?;

        Ok(MousePosition {
            x: reply.root_x() as _,
            y: reply.root_y() as _,
        })
    }

    fn set_pointer_position(&self, pos: MousePosition) -> Result<(), Self::Error> {
        self.conn
            .check_request(self.conn.send_request_checked(&WarpPointer {
                src_window: Window::none(),
                dst_window: self.root,
                src_x: 0,
                src_y: 0,
                src_width: 0,
                src_height: 0,
                dst_x: pos.x as _,
                dst_y: pos.y as _,
            }))
            .map_err(Into::into)
    }

    fn toggle_mouse(&self, button: MouseButton, down: bool) -> Result<(), Self::Error> {
        let dpy = self.conn.get_raw_dpy();

        unsafe {
            XTestFakeButtonEvent(dpy, button.id(), if down { 1 } else { 0 }, 0);
            XSync(dpy, 0);
        }

        Ok(())
    }

    fn scroll_mouse(&self, scroll: MouseScroll) -> Result<(), Self::Error> {
        let vert_button = if scroll.y > 0 {
            MouseButton::ScrollUp
        } else {
            MouseButton::ScrollDown
        }
        .id();
        let horiz_button = if scroll.x > 0 {
            MouseButton::ScrollRight
        } else {
            MouseButton::ScrollLeft
        }
        .id();

        let dpy = self.conn.get_raw_dpy();

        for _ in 0 .. scroll.y.abs() {
            unsafe {
                XTestFakeButtonEvent(dpy, vert_button, 1, 0);
                XTestFakeButtonEvent(dpy, vert_button, 0, 0);
            }
        }

        for _ in 0 .. scroll.x.abs() {
            unsafe {
                XTestFakeButtonEvent(dpy, horiz_button, 1, 0);
                XTestFakeButtonEvent(dpy, horiz_button, 0, 0);
            }
        }

        unsafe {
            XSync(dpy, 0);
        }

        Ok(())
    }

    fn clipboard_types(&self) -> Result<Vec<ClipboardType>, Self::Error> {
        Ok(vec![ClipboardType::Text])
    }

    fn clipboard_content(&self, type_name: ClipboardType) -> Result<Vec<u8>, Self::Error> {
        let atoms = &self.clipboard.setter.atoms;
        let target = match type_name {
            ClipboardType::Text => atoms.utf8_string,
            #[allow(unreachable_patterns)]
            _ => return Err(Error::UnsupportedClipboardType(type_name))
        };
        self.clipboard.load(
            atoms.clipboard,
            target,
            atoms.property,
            Duration::from_millis(50)
        ).map_err(Into::into)
    }

    fn set_clipboard_content(
        &mut self,
        type_name: ClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error> {
        let atoms = &self.clipboard.setter.atoms;
        let target = match type_name {
            ClipboardType::Text => atoms.utf8_string,
            #[allow(unreachable_patterns)]
            _ => return Err(Error::UnsupportedClipboardType(type_name))
        };
        self.clipboard.store(atoms.clipboard, target, content).map_err(Into::into)
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
        self.update_monitors()?;
        Ok(self
            .monitors
            .iter()
            .enumerate()
            .map(|(id, info)| Monitor {
                id: id as u32,
                name: info.name.clone(),
                width: info.width,
                height: info.height,
            })
            .collect())
    }

    fn windows(&mut self) -> Result<Vec<api::Window>, Self::Error> {
        let mut windows = Vec::new();
        self.list_windows(self.root, &mut windows)?;
        let mut api_windows = Vec::with_capacity(windows.len());
        for window in windows {
            let reply = self
                .conn
                .wait_for_reply(self.conn.send_request(&GetProperty {
                    delete: false,
                    window,
                    property: ATOM_WM_NAME,
                    r#type: ATOM_STRING,
                    long_offset: 0,
                    long_length: 100,
                }))?;

            if reply.length() == 0 {
                continue;
            }

            let name = str::from_utf8(reply.value())
                .map(|string| string.to_owned())
                .unwrap_or(String::from("unknown"));

            let geometry = self
                .conn
                .wait_for_reply(self.conn.send_request(&GetGeometry {
                    drawable: Drawable::Window(window),
                }))?;

            api_windows.push(api::Window {
                id: window.resource_id(),
                name,
                width: geometry.width() as u32,
                height: geometry.height() as u32,
            });
        }

        Ok(api_windows)
    }

    fn capture_display_frame(&self, monitor: &Monitor) -> Result<Frame, Self::Error> {
        let info = match self.monitors.get(monitor.id as usize) {
            Some(info) => info,
            None => return Err(Error::UnknownMonitor),
        };

        self.capture(info.x, info.y, info.width, info.height)
    }

    fn update_display_frame(
        &self,
        monitor: &Monitor,
        frame: &mut Frame,
    ) -> Result<(), Self::Error> {
        let info = match self.monitors.get(monitor.id as usize) {
            Some(info) => info,
            None => return Err(Error::UnknownMonitor),
        };

        self.update_frame(info.x, info.y, info.width, info.height, frame)
    }

    fn capture_window_frame(&self, window: &api::Window) -> Result<Frame, Self::Error> {
        let geometry = self
            .conn
            .wait_for_reply(self.conn.send_request(&GetGeometry {
                drawable: Drawable::Window(unsafe { Window::new(window.id) }),
            }))?;

        self.capture(
            geometry.x() as u32,
            geometry.y() as u32,
            geometry.width() as u32,
            geometry.height() as u32,
        )
    }

    fn update_window_frame(
        &self,
        window: &api::Window,
        frame: &mut Frame,
    ) -> Result<(), Self::Error> {
        let geometry = self
            .conn
            .wait_for_reply(self.conn.send_request(&GetGeometry {
                drawable: Drawable::Window(unsafe { Window::new(window.id) }),
            }))?;

        self.update_frame(
            geometry.x() as u32,
            geometry.y() as u32,
            geometry.width() as u32,
            geometry.height() as u32,
            frame,
        )
    }
}

impl X11Api {
    fn init_shm(conn: &Connection, size: size_t) -> Result<(c_int, *mut u32, Seg), Error> {
        let shmid = unsafe {
            shmget(
                IPC_PRIVATE,       // Dummy ID when making a new object
                size,              // Size of the object
                IPC_CREAT | 0o600, // Create a new object restricted to the current user
            )
        };

        if shmid == -1 {
            return Err(Error::ShmInit(errno()));
        }

        let shmaddr = unsafe {
            shmat(
                shmid,       // The ID of the shared memory object
                ptr::null(), // We don't want to attach it to an address, so we provide a null ptr
                0,           // No flags, we just want to get the address, not modify the object
            ) as *mut u32
        };

        // if shmaddr == (void*)-1
        if shmaddr as *mut c_void == usize::MAX as *mut c_void {
            let err = Error::ShmAttach(errno());

            // Make a best effort to release the resource. If this fails there's not much we can do
            unsafe {
                let _ = Self::mark_shm_for_deletion(shmid);
            }

            return Err(err);
        }

        let shmseg: Seg = conn.generate_id();
        let cookie = conn.send_request_checked(&Attach {
            shmseg,
            shmid: shmid as u32,
            read_only: false,
        });

        if let Err(err) = conn.check_request(cookie) {
            // Make a best effort to release resources
            unsafe {
                let _ = Self::mark_shm_for_deletion(shmid);
                let _ = Self::detach_shmaddr(shmaddr);
            }

            return Err(err.into());
        }

        Ok((shmid, shmaddr, shmseg))
    }

    fn capture(&self, x: u32, y: u32, width: u32, height: u32) -> Result<Frame, Error> {
        let offset = self.update_shm(x, y, width, height)?;

        let len = width as usize * height as usize;
        let mut buf: Vec<u8> = Vec::with_capacity(len * 3);

        unsafe {
            Self::copy_rgb(self.shmaddr.add(offset), buf.as_mut_ptr(), len);
            buf.set_len(len * 3);
        }

        Ok(RgbImage::from_vec(width, height, buf).expect("buf does not match width and height"))
    }

    fn update_frame(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        frame: &mut Frame,
    ) -> Result<(), Error> {
        let len = self.width as usize * self.height as usize;
        let data = &mut **frame;

        if data.len() != len * 3 {
            *frame = self.capture(x, y, width, height)?;
            return Ok(());
        }

        let offset = self.update_shm(x, y, width, height)?;

        unsafe {
            Self::copy_rgb(self.shmaddr.add(offset), data.as_mut_ptr(), len);
        }

        Ok(())
    }

    fn update_shm(&self, x: u32, y: u32, width: u32, height: u32) -> Result<usize, Error> {
        let offset = (y * self.width as u32 + x) * self.depth as u32;
        let cookie = self.conn.send_request(&GetImage {
            drawable: Drawable::Window(self.root),
            x: x as _,
            y: y as _,
            width: width as _,
            height: height as _,
            plane_mask: u32::MAX, // All planes
            format: 2,            // ZPixmap
            shmseg: self.shmseg,
            offset,
        });
        let _reply = self.conn.wait_for_reply(cookie)?;
        Ok(offset as usize)
    }

    #[inline]
    unsafe fn mark_shm_for_deletion(id: c_int) -> Result<(), Error> {
        if shmctl(id, IPC_RMID, ptr::null_mut()) != 0 {
            return Err(Error::ShmRmid(errno()));
        }

        Ok(())
    }

    #[inline]
    unsafe fn detach_shmaddr(shmaddr: *mut u32) -> Result<(), Error> {
        if shmdt(shmaddr as *mut _ as *const _) != 0 {
            return Err(Error::ShmDetach(errno()));
        }

        Ok(())
    }

    fn release_shm(conn: &Connection, shmid: c_int, shmaddr: *mut u32, shmseg: Seg) {
        // TODO: better error handling?

        let cookie = conn.send_request_checked(&Detach { shmseg });
        let _ = conn.check_request(cookie);

        unsafe {
            let _ = Self::mark_shm_for_deletion(shmid);
            let _ = Self::detach_shmaddr(shmaddr);
        }
    }

    fn update_monitors(&mut self) -> Result<(), Error> {
        let monitors = self
            .conn
            .wait_for_reply(self.conn.send_request(&GetMonitors {
                window: self.root,
                get_active: true,
            }))?;

        let mut monitor_list = Vec::with_capacity(monitors.length() as usize);

        for (monitor_info, reply) in monitors.monitors().map(|info| {
            (
                info,
                self.conn
                    .wait_for_reply(self.conn.send_request(&GetAtomName { atom: info.name() })),
            )
        }) {
            monitor_list.push(X11MonitorInfo {
                name: reply?.name().unwrap_or("unknown").to_owned(),
                x: monitor_info.x() as u32,
                y: monitor_info.y() as u32,
                width: monitor_info.width() as u32,
                height: monitor_info.height() as u32,
            });
        }

        self.monitors = monitor_list;
        Ok(())
    }

    fn list_windows(&self, window: Window, windows: &mut Vec<Window>) -> Result<(), Error> {
        let wininfo = self
            .conn
            .wait_for_reply(self.conn.send_request(&GetWindowAttributes { window }))?;

        if wininfo.map_state() == MapState::Viewable {
            windows.push(window);
        }

        let tree_query_reply = self
            .conn
            .wait_for_reply(self.conn.send_request(&QueryTree { window }))?;

        for &child in tree_query_reply.children() {
            self.list_windows(child, windows)?;
        }

        Ok(())
    }

    #[inline]
    unsafe fn copy_rgb(mut src: *const u32, mut dst: *mut u8, len: usize) {
        for _ in 0 .. len {
            let [b, g, r, _a] = (*src).to_le_bytes();
            *(dst as *mut [u8; 3]) = [r, g, b];
            src = src.add(1);
            dst = dst.add(3);
        }
    }
}

impl Finalize for X11Api {}

impl Drop for X11Api {
    fn drop(&mut self) {
        Self::release_shm(&self.conn, self.shmid, self.shmaddr, self.shmseg);
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to open display")]
    DisplayOpenFailed,
    #[error("internal xcb error: {0}")]
    XcbError(#[from] XcbError),
    #[error("failed to initialize shared memory object: error code {0}")]
    ShmInit(Errno),
    #[error("failed to attach to shared memory object: error code {0}")]
    ShmAttach(Errno),
    #[error("failed to detach from shared memory object: error code {0}")]
    ShmDetach(Errno),
    #[error("failed to mark shared memory object for deletion: error code {0}")]
    ShmRmid(Errno),
    #[error("unknown monitor")]
    UnknownMonitor,
    #[error("clipboard error: {0}")]
    ClipboardError(#[from] X11ClipboardError),
    #[error("clipboard type {0:?} not supported")]
    UnsupportedClipboardType(ClipboardType)
}

// TODO: get this sorted out
unsafe impl Send for Error {}

impl From<xcb::Error> for Error {
    fn from(error: xcb::Error) -> Self {
        Self::XcbError(XcbError(error))
    }
}

impl From<ConnError> for Error {
    fn from(error: ConnError) -> Self {
        Self::XcbError(XcbError(xcb::Error::Connection(error)))
    }
}

impl From<ProtocolError> for Error {
    fn from(error: ProtocolError) -> Self {
        Self::XcbError(XcbError(xcb::Error::Protocol(error)))
    }
}

#[derive(Debug)]
pub struct XcbError(pub xcb::Error);

impl fmt::Display for XcbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl StdError for XcbError {}
