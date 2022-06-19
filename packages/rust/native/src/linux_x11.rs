use errno::{errno, Errno};
use image::RgbImage;
use libc::{c_int, shmat, shmctl, shmdt, shmget, size_t, IPC_CREAT, IPC_PRIVATE, IPC_RMID};
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Formatter},
    ptr,
    str,
    time::Duration,
};
use x11::{
    xlib::{XDefaultRootWindow, XKeysymToKeycode, XOpenDisplay, XRaiseWindow, XSync},
    xtest::{XTestFakeButtonEvent, XTestFakeKeyEvent},
};
use x11_clipboard::{error::Error as X11ClipboardError, Clipboard};
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

use crate::api::{self, *};

struct X11MonitorInfo {
    id: MonitorId,
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
    capture_info: Option<CaptureInfo>,

    // Monitor map
    monitors: Vec<X11MonitorInfo>,

    // Clipboard API
    clipboard: Clipboard,
}

unsafe impl Send for X11Api {}

impl X11Api {
    pub fn new() -> Result<Self, Error> {
        let dpy = unsafe { XOpenDisplay(ptr::null()) };
        if dpy.is_null() {
            return Err(Error::DisplayOpenFailed);
        }
        let root = unsafe { Window::new(XDefaultRootWindow(dpy) as u32) };
        let conn = unsafe { Connection::from_xlib_display(dpy) };

        Ok(Self {
            conn,
            root,
            capture_info: None,
            monitors: Vec::new(),
            clipboard: Clipboard::new()?,
        })
    }
}

impl NativeApiTemplate for X11Api {
    type Error = Error;

    fn key_toggle(&mut self, keysym: u32, down: bool) -> Result<(), Error> {
        let dpy = self.conn.get_raw_dpy();
        let down = if down { 1 } else { 0 };

        unsafe {
            let keycode = XKeysymToKeycode(dpy, keysym as _);
            XTestFakeKeyEvent(dpy, keycode as _, down, 0);
            XSync(dpy, 0);
        }

        Ok(())
    }

    fn pointer_position(&mut self, windows: &[WindowId]) -> Result<MousePosition, Error> {
        let reply = self
            .conn
            .wait_for_reply(self.conn.send_request(&QueryPointer { window: self.root }))?;
        let x = reply.root_x() as u32;
        let y = reply.root_y() as u32;
        let monitor_id = self
            .monitors
            .iter()
            .find(|info| Self::in_aabb(x, y, info.x, info.y, info.width, info.height))
            .map(|info| info.id)
            .unwrap_or(0);

        let window_requests = windows
            .iter()
            .copied()
            .map(|window| {
                (
                    window,
                    self.conn.send_request(&GetGeometry {
                        drawable: Drawable::Window(unsafe { Window::new(window) }),
                    }),
                )
            })
            .collect::<Vec<_>>();

        Ok(MousePosition {
            x,
            y,
            monitor_id,
            window_relatives: window_requests
                .into_iter()
                .flat_map(|(window_id, cookie)| {
                    self.conn
                        .wait_for_reply(cookie)
                        .map(|reply| {
                            (
                                reply.x() as u32,
                                reply.y() as u32,
                                reply.width() as u32,
                                reply.height() as u32,
                            )
                        })
                        .map(|(wx, wy, w, h)| {
                            Self::in_aabb(x, y, wx, wy, w, h).then(|| PointerPositionRelative {
                                x: x - wx,
                                y: y - wy,
                                window_id,
                            })
                        })
                        .transpose()
                })
                .collect::<Result<_, _>>()?,
        })
    }

    fn set_pointer_position_absolute(
        &mut self,
        x: u32,
        y: u32,
        monitor_id: MonitorId,
    ) -> Result<(), Self::Error> {
        let &X11MonitorInfo {
            x: monitor_x,
            y: monitor_y,
            ..
        } = self.get_monitor(monitor_id)?;

        self.conn
            .check_request(self.conn.send_request_checked(&WarpPointer {
                src_window: Window::none(),
                dst_window: self.root,
                src_x: 0,
                src_y: 0,
                src_width: 0,
                src_height: 0,
                dst_x: (monitor_x + x) as _,
                dst_y: (monitor_y + y) as _,
            }))
            .map_err(Into::into)
    }

    fn set_pointer_position_relative(
        &mut self,
        x: u32,
        y: u32,
        window_id: WindowId,
    ) -> Result<(), Self::Error> {
        self.conn
            .check_request(self.conn.send_request_checked(&WarpPointer {
                src_window: Window::none(),
                dst_window: unsafe { Window::new(window_id) },
                src_x: 0,
                src_y: 0,
                src_width: 0,
                src_height: 0,
                dst_x: x as _,
                dst_y: y as _,
            }))
            .map_err(Into::into)
    }

    fn toggle_mouse(
        &mut self,
        button: MouseButton,
        down: bool,
        window_id: Option<WindowId>,
    ) -> Result<(), Error> {
        let dpy = self.conn.get_raw_dpy();

        unsafe {
            if let Some(window) = window_id {
                XRaiseWindow(dpy, window as u64);
            }

            XTestFakeButtonEvent(dpy, button.id(), if down { 1 } else { 0 }, 0);
            XSync(dpy, 0);
        }

        Ok(())
    }

    fn clipboard_content(&mut self, type_name: &ClipboardType) -> Result<Option<Vec<u8>>, Error> {
        let atoms = &self.clipboard.setter.atoms;
        let target = match type_name {
            ClipboardType::Text => atoms.utf8_string,
            #[allow(unreachable_patterns)]
            _ => return Err(Error::UnsupportedClipboardType(type_name.clone())),
        };
        self.clipboard
            .load(
                atoms.clipboard,
                target,
                atoms.property,
                Duration::from_secs(1),
            )
            .map(Some)
            .map_err(Into::into)
    }

    fn set_clipboard_content(
        &mut self,
        type_name: &ClipboardType,
        content: &[u8],
    ) -> Result<(), Error> {
        let atoms = &self.clipboard.setter.atoms;
        let target = match type_name {
            ClipboardType::Text => atoms.utf8_string,
            #[allow(unreachable_patterns)]
            _ => return Err(Error::UnsupportedClipboardType(type_name.clone())),
        };
        self.clipboard
            .store(atoms.clipboard, target, content)
            .map_err(Into::into)
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Error> {
        self.update_monitors()?;
        Ok(self
            .monitors
            .iter()
            .map(|info| Monitor {
                id: info.id,
                name: info.name.clone(),
                width: info.width,
                height: info.height,
            })
            .collect())
    }

    fn windows(&mut self) -> Result<Vec<api::Window>, Error> {
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
                .unwrap_or_else(|_| String::from("unknown"));

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

    fn capture_monitor_frame(&mut self, monitor_id: u32) -> Result<Frame, Error> {
        let &X11MonitorInfo {
            x,
            y,
            width,
            height,
            ..
        } = self.get_monitor(monitor_id)?;
        self.capture(self.root, x, y, width, height)
    }

    fn update_monitor_frame(&mut self, monitor_id: u32, frame: &mut Frame) -> Result<(), Error> {
        let &X11MonitorInfo {
            x,
            y,
            width,
            height,
            ..
        } = self.get_monitor(monitor_id)?;
        self.update_frame(self.root, x, y, width, height, frame)
    }

    fn capture_window_frame(&mut self, window_id: u32) -> Result<Frame, Error> {
        let x11_window = unsafe { Window::new(window_id) };
        let geometry = self
            .conn
            .wait_for_reply(self.conn.send_request(&GetGeometry {
                drawable: Drawable::Window(x11_window),
            }))?;

        self.capture(
            x11_window,
            0,
            0,
            geometry.width() as u32,
            geometry.height() as u32,
        )
    }

    fn update_window_frame(&mut self, window_id: u32, frame: &mut Frame) -> Result<(), Error> {
        let x11_window = unsafe { Window::new(window_id) };
        let geometry = self
            .conn
            .wait_for_reply(self.conn.send_request(&GetGeometry {
                drawable: Drawable::Window(x11_window),
            }))?;

        self.update_frame(
            x11_window,
            0,
            0,
            geometry.width() as u32,
            geometry.height() as u32,
            frame,
        )
    }
}

impl X11Api {
    fn lazy_init_capture(&mut self) -> Result<CaptureInfo, Error> {
        if self.capture_info.is_none() {
            self.capture_info = Self::init_capture_info(&self.conn, self.root).map(Some)?;
        }

        Ok(self.capture_info.unwrap())
    }

    fn init_capture_info(conn: &Connection, root: Window) -> Result<CaptureInfo, Error> {
        let cookie = conn.send_request(&GetGeometry {
            drawable: Drawable::Window(root),
        });
        let reply = conn.wait_for_reply(cookie)?;

        let width = reply.width();
        let height = reply.height();
        let depth = reply.depth() as size_t;

        let (shmid, shmaddr, shmseg) =
            Self::init_shm(&conn, width as size_t * height as size_t * depth)?;

        Ok(CaptureInfo {
            width,
            height,
            shmid,
            shmaddr,
            shmseg,
        })
    }

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
        if ptr::eq(shmaddr, usize::MAX as *mut _) {
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

    fn capture(
        &mut self,
        window: Window,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<Frame, Error> {
        let info = self.lazy_init_capture()?;

        self.update_shm(info.shmseg, window, x, y, width, height)?;

        let len = width as usize * height as usize;
        let mut buf: Vec<u8> = Vec::with_capacity(len * 3);

        unsafe {
            Self::copy_rgb(info.shmaddr, buf.as_mut_ptr(), len);
            buf.set_len(len * 3);
        }

        Ok(RgbImage::from_vec(width, height, buf).expect("buf does not match width and height"))
    }

    fn update_frame(
        &mut self,
        window: Window,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        frame: &mut Frame,
    ) -> Result<(), Error> {
        let info = self.lazy_init_capture()?;

        let len = info.width as usize * info.height as usize;
        let data = &mut **frame;

        if data.len() != len * 3 {
            *frame = self.capture(window, x, y, width, height)?;
            return Ok(());
        }

        self.update_shm(info.shmseg, window, x, y, width, height)?;

        unsafe {
            Self::copy_rgb(info.shmaddr, data.as_mut_ptr(), len);
        }

        Ok(())
    }

    fn update_shm(
        &self,
        shmseg: Seg,
        window: Window,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        let cookie = self.conn.send_request(&GetImage {
            drawable: Drawable::Window(window),
            x: x as _,
            y: y as _,
            width: width as _,
            height: height as _,
            plane_mask: u32::MAX, // All planes
            format: 2,            // ZPixmap
            shmseg,
            offset: 0,
        });
        let _reply = self.conn.wait_for_reply(cookie)?;
        Ok(())
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
                id: monitor_info.name().resource_id(),
                name: reply?.name().to_string(),
                x: monitor_info.x() as u32,
                y: monitor_info.y() as u32,
                width: monitor_info.width() as u32,
                height: monitor_info.height() as u32,
            });
        }

        self.monitors = monitor_list;
        Ok(())
    }

    fn get_monitor(&mut self, id: MonitorId) -> Result<&X11MonitorInfo, Error> {
        // We have to use indices here because for once the programmer is smarter than the borrow
        // checker

        if let Some(index) = self.get_cached_monitor_index(id) {
            return Ok(&self.monitors[index]);
        }

        self.update_monitors()?;
        self.get_cached_monitor_index(id)
            .map(|index| &self.monitors[index])
            .ok_or(Error::UnknownMonitor)
    }

    fn get_cached_monitor_index(&self, id: MonitorId) -> Option<usize> {
        self.monitors.iter().position(|monitor| monitor.id == id)
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

    #[inline]
    fn in_aabb(x1: u32, y1: u32, x2: u32, y2: u32, w: u32, h: u32) -> bool {
        x1 >= x2 && y1 >= y2 && (x1 - x2) < w && (y1 - y2) < h
    }
}

impl Drop for X11Api {
    fn drop(&mut self) {
        if let Some(info) = self.capture_info.as_ref() {
            Self::release_shm(&self.conn, info.shmid, info.shmaddr, info.shmseg);
        }
    }
}

#[derive(Clone, Copy)]
struct CaptureInfo {
    width: u16,
    height: u16,
    shmid: c_int,
    shmaddr: *mut u32,
    shmseg: Seg,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to open display")]
    DisplayOpenFailed,
    #[error("internal xcb error: {0}")]
    Xcb(#[from] XcbError),
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
    Clipboard(#[from] X11ClipboardError),
    #[error("clipboard type {0:?} not supported")]
    UnsupportedClipboardType(ClipboardType),
}

// TODO: get this sorted out
unsafe impl Send for Error {}

impl From<xcb::Error> for Error {
    fn from(error: xcb::Error) -> Self {
        Self::Xcb(XcbError(error))
    }
}

impl From<ConnError> for Error {
    fn from(error: ConnError) -> Self {
        Self::Xcb(XcbError(xcb::Error::Connection(error)))
    }
}

impl From<ProtocolError> for Error {
    fn from(error: ProtocolError) -> Self {
        Self::Xcb(XcbError(xcb::Error::Protocol(error)))
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
