use crate::api::*;
use errno::{errno, Errno};
use image::RgbImage;
use libc::{c_int, c_void, shmat, shmctl, shmdt, shmget, size_t, IPC_CREAT, IPC_PRIVATE, IPC_RMID};
use neon::prelude::Finalize;
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
    ptr,
};
use x11::{
    xlib::{XDefaultRootWindow, XKeysymToKeycode, XOpenDisplay, XSync},
    xtest::XTestFakeKeyEvent,
};
use xcb::{
    shm::{Attach, Detach, GetImage, Seg},
    x::{Drawable, GetGeometry, QueryPointer, Window},
    xkb::{MAJOR_VERSION, MINOR_VERSION},
    ConnError,
    Connection,
    ProtocolError,
    XidNew,
};
use xkbcommon_sys::{
    xkb_context,
    xkb_context_flags::XKB_CONTEXT_NO_FLAGS,
    xkb_context_new,
    xkb_context_unref,
    xkb_keymap,
    xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
    xkb_keymap_unref,
    xkb_state,
    xkb_state_unref,
    xkb_x11_get_core_keyboard_device_id,
    xkb_x11_keymap_new_from_device,
    xkb_x11_setup_xkb_extension,
    xkb_x11_setup_xkb_extension_flags::XKB_X11_SETUP_XKB_EXTENSION_NO_FLAGS,
    xkb_x11_state_new_from_device,
};

pub struct Capture {
    // General fields
    conn: Connection,
    root: Window,

    // Screen capture fields
    width: u16,
    height: u16,
    shmid: c_int,
    shmaddr: *mut u32,
    shmseg: Seg,

    // XKB fields
    xkb_context: *mut xkb_context,
    xkb_keymap: *mut xkb_keymap,
    xkb_state: *mut xkb_state,
}

unsafe impl Send for Capture {}

impl NativeCapture for Capture {
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

        let (xkb_context, xkb_keymap, xkb_state) = match Self::init_xkb(&conn) {
            Ok((ctx, map, state)) => (ctx, map, state),
            Err(err) => {
                Self::release_shm(&conn, shmid, shmaddr, shmseg);
                return Err(err.into());
            }
        };

        Ok(Self {
            conn,
            root,
            width,
            height,
            shmid,
            shmaddr,
            shmseg,
            xkb_context,
            xkb_keymap,
            xkb_state,
        })
    }

    fn capture_screen(&self) -> Result<RgbImage, Self::Error> {
        self.update_shm()?;

        let len = self.width as usize * self.height as usize * 3;
        let mut buf: Vec<u8> = Vec::with_capacity(len);

        unsafe {
            Self::copy_rgb(self.shmaddr, buf.as_mut_ptr(), len / 3);
            buf.set_len(len);
        }

        Ok(
            RgbImage::from_vec(self.width as u32, self.height as u32, buf)
                .expect("buf does not match width and height"),
        )
    }

    fn update_screen_capture(&self, cap: &mut RgbImage) -> Result<(), Self::Error> {
        let len = self.width as usize * self.height as usize * 3;
        let data = &mut **cap;

        if data.len() != len {
            *cap = self.capture_screen()?;
            return Ok(());
        }

        self.update_shm()?;

        unsafe {
            Self::copy_rgb(self.shmaddr, data.as_mut_ptr(), len / 3);
        }

        Ok(())
    }

    fn pointer_position(&self) -> Result<(u32, u32), Self::Error> {
        let reply = self
            .conn
            .wait_for_reply(self.conn.send_request(&QueryPointer { window: self.root }))?;

        Ok((reply.root_x() as u32, reply.root_y() as u32))
    }

    fn key_toggle(&self, keysym: u32) {
        let dpy = self.conn.get_raw_dpy();

        unsafe {
            let keycode = XKeysymToKeycode(dpy, keysym as _);
            XTestFakeKeyEvent(dpy, keycode as _, 1, 0);
            XTestFakeKeyEvent(dpy, keycode as _, 0, 0);
            XSync(dpy, 0);
        }
    }
}

impl Capture {
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

    fn update_shm(&self) -> Result<(), Error> {
        let cookie = self.conn.send_request(&GetImage {
            drawable: Drawable::Window(self.root),
            x: 0,
            y: 0,
            width: self.width as u16,
            height: self.height as u16,
            plane_mask: u32::MAX, // All planes
            format: 2,            // ZPixmap
            shmseg: self.shmseg,
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

    fn init_xkb(
        conn: &Connection,
    ) -> Result<(*mut xkb_context, *mut xkb_keymap, *mut xkb_state), Error> {
        let raw_conn = conn.get_raw_conn() as *mut xkbcommon_sys::xcb_connection_t;

        let res = unsafe {
            xkb_x11_setup_xkb_extension(
                raw_conn,
                MAJOR_VERSION as u16,
                MINOR_VERSION as u16,
                XKB_X11_SETUP_XKB_EXTENSION_NO_FLAGS,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if res == 0 {
            return Err(Error::XkbInit);
        }

        let context = unsafe { xkb_context_new(XKB_CONTEXT_NO_FLAGS) };
        if context.is_null() {
            return Err(Error::XkbContextInit);
        }

        let device_id = unsafe { xkb_x11_get_core_keyboard_device_id(raw_conn) };
        if device_id == -1 {
            unsafe {
                xkb_context_unref(context);
            }
            return Err(Error::XkbGetCoreKbDev);
        }

        let keymap = unsafe {
            xkb_x11_keymap_new_from_device(
                context,
                raw_conn,
                device_id,
                XKB_KEYMAP_COMPILE_NO_FLAGS,
            )
        };
        if keymap.is_null() {
            unsafe {
                xkb_context_unref(context);
            }
            return Err(Error::XkbFetchKeymap);
        }

        let state = unsafe { xkb_x11_state_new_from_device(keymap, raw_conn, device_id) };
        if state.is_null() {
            unsafe {
                xkb_keymap_unref(keymap);
                xkb_context_unref(context);
            }
            return Err(Error::XkbNewState);
        }

        Ok((context, keymap, state))
    }

    fn release_xkb(context: *mut xkb_context, keymap: *mut xkb_keymap, state: *mut xkb_state) {
        unsafe {
            xkb_state_unref(state);
            xkb_keymap_unref(keymap);
            xkb_context_unref(context);
        }
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

impl Finalize for Capture {}

impl Drop for Capture {
    fn drop(&mut self) {
        Self::release_shm(&self.conn, self.shmid, self.shmaddr, self.shmseg);
        Self::release_xkb(self.xkb_context, self.xkb_keymap, self.xkb_state);
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to open display")]
    DisplayOpenFailed,
    #[error("internal xcb error: {0}")]
    XcbError(#[from] XcbError),
    #[error("failed to map screen number to screen object")]
    ScreenNumMismatch,
    #[error("failed to initialize shared memory object: error code {0}")]
    ShmInit(Errno),
    #[error("failed to attach to shared memory object: error code {0}")]
    ShmAttach(Errno),
    #[error("failed to detach from shared memory object: error code {0}")]
    ShmDetach(Errno),
    #[error("failed to mark shared memory object for deletion: error code {0}")]
    ShmRmid(Errno),
    #[error("failed to setup xkb extension")]
    XkbInit,
    #[error("failed to initialize xkb context")]
    XkbContextInit,
    #[error("failed to fetch core keyboard device ID")]
    XkbGetCoreKbDev,
    #[error("failed to fetch keymap")]
    XkbFetchKeymap,
    #[error("failed to create new state for device")]
    XkbNewState,
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

impl Display for XcbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl StdError for XcbError {}
