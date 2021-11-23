use neon::prelude::*;
use x11::xlib::*;
use std::{ptr, slice};
use std::cell::Cell;
use std::mem::MaybeUninit;
use crate::api::*;
use thiserror::Error;

pub struct X11Image {
    img: *mut XImage
}

unsafe impl Send for X11Image { }

impl X11Image {
    fn new(img: *mut XImage) -> Self {
        Self { img }
    }
}

impl Image for X11Image {
    fn width(&self) -> usize {
        unsafe { (*self.img).width as usize }
    }

    fn height(&self) -> usize {
        unsafe { (*self.img).height as usize }
    }

    fn pixels(&self) -> &[Pixel] {
        let ptr = unsafe { (*self.img).data };
        unsafe {
            // We know that the data is in RGB format with a trailing zero pixel as padding,
            // so we can essentially pretend that the data array is a Pixel array
            slice::from_raw_parts(
                ptr as *const _ as *const Pixel,
                self.width() * self.height()
            )
        }
    }
}

impl Drop for X11Image {
    fn drop(&mut self) {
        unsafe {
            XDestroyImage(self.img);
        }
    }
}

pub struct X11ScreenHandle {
    display: Cell<*mut Display>,
    root: Drawable,
}

unsafe impl Send for X11ScreenHandle {}

impl ScreenHandle for X11ScreenHandle {
    type Image = X11Image;
    type Error = X11Error;

    fn new() -> Result<Self, Self::Error> {
        let display = unsafe { XOpenDisplay(ptr::null()) };
        if display.is_null() {
            return Err(X11Error::DisplayOpenFailed);
        }

        let root = unsafe { XDefaultRootWindow(display) };
        Ok(Self {
            display: Cell::new(display),
            root,
        })
    }

    fn capture(&mut self) -> Result<Self::Image, Self::Error> {
        let display = self.display.get();
        if display.is_null() {
            return Err(X11Error::HandleClosed);
        }

        let mut window_attrs = MaybeUninit::uninit();
        let result = unsafe {
            XGetWindowAttributes(display, self.root, window_attrs.as_mut_ptr())
        };
        if result == 0 {
            return Err(X11Error::GetAttr);
        }

        let window_attrs = unsafe { window_attrs.assume_init() };
        let width = window_attrs.width as u32;
        let height = window_attrs.height as u32;
        let img_ptr = unsafe { XGetImage(display, self.root, 0, 0, width, height, u64::MAX, ZPixmap) };
        if img_ptr.is_null() {
            return Err(X11Error::GetImage);
        }

        Ok(X11Image::new(img_ptr))
    }
    
    fn close(&mut self) {
        let display = self.display.replace(ptr::null_mut());
        if !display.is_null() {
            unsafe { XCloseDisplay(display); }
        }
    }
}

impl Finalize for X11ScreenHandle {
    fn finalize<'a, C: Context<'a>>(self, _: &mut C) { }
}

impl Drop for X11ScreenHandle {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Error, Debug)]
pub enum X11Error {
    #[error("failed to open display handle")]
    DisplayOpenFailed,
    #[error("attempted to use an already closed screen handle")]
    HandleClosed,
    #[error("failed to get window attributes")]
    GetAttr,
    #[error("failed to capture screen")]
    GetImage
}

