use neon::prelude::*;
use xcb::{ConnError, Connection, ProtocolError, shm::{
        Seg,
        Attach,
        Detach,
        GetImage,
    }, x::{GetGeometry, Drawable, Window}};
use std::ptr;
use std::fmt::{self, Display, Debug, Formatter};
use std::error::Error;
use thiserror::Error;
use image::RgbImage;
use libc::{IPC_CREAT, IPC_PRIVATE, IPC_RMID, c_int, c_void, shmat, shmctl, shmdt, shmget, size_t};
use errno::{Errno, errno};

pub struct ScreenHandle {
    conn: Connection,
    root: Window,
    width: usize,
    height: usize,
    shmid: c_int,
    shmaddr: *mut u32,
    shmseg: Seg,
}

unsafe impl Send for ScreenHandle {}

impl ScreenHandle {
    pub fn new() -> Result<Self, ScreencapError> {
        let (conn, screen_num) = Connection::connect(None)?;
        let root = conn
            .get_setup()
            .roots()
            .nth(screen_num as usize)
            .ok_or(ScreencapError::ScreenNumMismatch)?
            .root();
        
        let cookie = conn.send_request(&GetGeometry {
            drawable: Drawable::Window(root)
        });
        let reply = conn.wait_for_reply(cookie)?;

        let width = reply.width() as size_t;
        let height = reply.height() as size_t;
        let depth = reply.depth() as size_t;

        let shmid = unsafe {
            shmget(
                IPC_PRIVATE, // Dummy ID when making a new object
                width * height * depth, // Size of the object
                IPC_CREAT | 0o600 // Create a new object restricted to the current user
            )
        };

        if shmid == -1 {
            return Err(ScreencapError::ShmInit(errno()));
        }

        let shmaddr = unsafe {
            shmat(
                shmid, // The ID of the shared memory object
                ptr::null(), // We don't want to attach it to an address, so we provide a null ptr
                0 // No flags, we just want to get the address, not modify the object
            ) as *mut u32
        };

        // if shmaddr == (void*)-1
        if shmaddr as *mut c_void == usize::MAX as *mut c_void {
            let err = ScreencapError::ShmAttach(errno());

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
            read_only: false
        });

        if let Err(err) = conn.check_request(cookie) {
            // Make a best effort to release resources
            unsafe {
                let _ = Self::mark_shm_for_deletion(shmid);
                let _ = Self::detach_shmaddr(shmaddr);
            }

            return Err(err.into());
        }
        
        Ok(Self {
            conn,
            root,
            width,
            height,
            shmid,
            shmaddr,
            shmseg,
        })
    }

    pub fn capture(&self) -> Result<RgbImage, ScreencapError> {
        self.update_shm()?;

        let len = self.width * self.height * 3;
        let mut buf: Vec<u8> = Vec::with_capacity(len);

        unsafe {
            Self::copy_rgb(self.shmaddr, buf.as_mut_ptr(), len / 3);
            buf.set_len(len);
        }

        Ok(
            RgbImage::from_vec(
                self.width as u32,
                self.height as u32,
                buf
            ).expect("buf does not match width and height")
        )
    }

    pub fn update(&self, image: &mut RgbImage) -> Result<(), ScreencapError> {
        let len = self.width * self.height * 3;
        let data = &mut **image;

        if data.len() != len {
            *image = self.capture()?;
            return Ok(());
        }

        self.update_shm()?;

        unsafe {
            Self::copy_rgb(self.shmaddr, data.as_mut_ptr(), len / 3);
        }

        Ok(())
    }

    fn update_shm(&self) -> Result<(), ScreencapError> {
        let cookie = self.conn.send_request(&GetImage {
            drawable: Drawable::Window(self.root),
            x: 0,
            y: 0,
            width: self.width as u16,
            height: self.height as u16,
            plane_mask: u32::MAX, // All planes
            format: 2, // ZPixmap
            shmseg: self.shmseg,
            offset: 0
        });
        let _reply = self.conn.wait_for_reply(cookie)?;
        Ok(())
    }

    #[inline]
    unsafe fn mark_shm_for_deletion(id: c_int) -> Result<(), ScreencapError> {
        if shmctl(id, IPC_RMID, ptr::null_mut()) != 0 {
            return Err(ScreencapError::ShmRmid(errno()));
        }

        Ok(())
    }

    #[inline]
    unsafe fn detach_shmaddr(shmaddr: *mut u32) -> Result<(), ScreencapError> {
        if shmdt(shmaddr as *mut _ as *const _) != 0 {
            return Err(ScreencapError::ShmDetach(errno()));
        }

        Ok(())
    }

    #[inline]
    unsafe fn copy_rgb(mut src: *const u32, mut dst: *mut u8, len: usize) {
        for _ in 0..len {
            let [b, g, r, _a] = (*src).to_le_bytes();
            *(dst as *mut [u8; 3]) = [r, g, b];
            src = src.add(1);
            dst = dst.add(3);
        }
    }
}

impl Finalize for ScreenHandle {}

impl Drop for ScreenHandle {
    fn drop(&mut self) {
        let cookie = self.conn.send_request_checked(&Detach {
            shmseg: self.shmseg
        });
        let xcb_res = self.conn.check_request(cookie);

        unsafe {
            let _ = Self::mark_shm_for_deletion(self.shmid);
            let _ = Self::detach_shmaddr(self.shmaddr);
        }

        // TODO: make this more graceful
        xcb_res.expect("failed to detach from window");
    }
}

#[derive(Error, Debug)]
pub enum ScreencapError {
    #[error("internal xcb error: {0}")]
    Internal(#[from] XcbError),
    #[error("failed to map screen number to screen object")]
    ScreenNumMismatch,
    #[error("failed to initialize shared memory object: error code {0}")]
    ShmInit(Errno),
    #[error("failed to attach to shared memory object: error code {0}")]
    ShmAttach(Errno),
    #[error("failed to detach from shared memory object: error code {0}")]
    ShmDetach(Errno),
    #[error("failed to mark shared memory object for deletion: error code {0}")]
    ShmRmid(Errno)
}

impl From<xcb::Error> for ScreencapError {
    fn from(error: xcb::Error) -> Self {
        Self::Internal(XcbError(error))
    }
}

impl From<ConnError> for ScreencapError {
    fn from(error: ConnError) -> Self {
        Self::Internal(XcbError(xcb::Error::Connection(error)))
    }
}

impl From<ProtocolError> for ScreencapError {
    fn from(error: ProtocolError) -> Self {
        Self::Internal(XcbError(xcb::Error::Protocol(error)))
    }
}

#[derive(Debug)]
pub struct XcbError(pub xcb::Error);

impl Display for XcbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for XcbError {}
