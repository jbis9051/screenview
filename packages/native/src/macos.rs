use std::borrow::Borrow;
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::ptr::null_mut;
use cocoa::appkit::*;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSString};
use core_foundation::base::{Boolean, FromVoid, TCFType, ToVoid};
use core_foundation::string::{CFString, CFStringGetCStringPtr, CFStringRef, kCFStringEncodingUTF8};
use core_graphics::display::{CFArray, CFArrayGetCount, CFArrayGetValueAtIndex, CFDictionary, CFDictionaryGetValueIfPresent, CFDictionaryRef, CGRect, kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly};
use core_graphics::window::{CGWindowListCopyWindowInfo, kCGWindowBounds, kCGWindowListOptionExcludeDesktopElements, kCGWindowName, kCGWindowOwnerName};
use libc::c_void;
use neon::prelude::Finalize;
use neon::types::StringOverflow;
use crate::api::*;

pub struct MacosApi;

impl Finalize for MacosApi {}

impl MacosApi {
    fn cgstring_to_string(cf_ref: CFStringRef) -> Option<String> {
        let c_ptr = unsafe { CFStringGetCStringPtr(cf_ref, kCFStringEncodingUTF8) };
        if c_ptr.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(c_ptr).to_str().unwrap().to_owned() })
    }
}

impl NativeApiTemplate for MacosApi {
    type Error = Error;

    fn new() -> Result<Self, Self::Error> {
        Ok(Self {})
    }

    fn key_toggle(&self, key: Key, down: bool) {
        unimplemented!()
    }

    fn pointer_position(&self) -> Result<MousePosition, Self::Error> {
        let point = unsafe {
            NSEvent::mouseLocation(nil)
        };
        Ok(
            MousePosition {
                x: point.x as u32,
                y: point.y as u32,
                monitor_id: 0,
            })
    }

    fn set_pointer_position(&self, pos: MousePosition) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn toggle_mouse(&self, button: MouseButton, down: bool) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn scroll_mouse(&self, scroll: MouseScroll) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn clipboard_types(&self) -> Result<Vec<ClipboardType>, Self::Error> {
        unimplemented!()
    }

    fn clipboard_content(&self, type_name: ClipboardType) -> Result<Vec<u8>, Self::Error> {
        unimplemented!()
    }

    fn set_clipboard_content(
        &mut self,
        type_name: ClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
        let display = unsafe {
            NSScreen::screens(nil)
        };
        let count = unsafe { NSArray::count(display) };
        let mut monitors = Vec::with_capacity(count as usize);
        for i in 0..count {
            let nsscreen = unsafe { NSArray::objectAtIndex(display, i) };
            let nsrect = unsafe { NSScreen::frame(nsscreen) };
            let name = unsafe { CStr::from_ptr(NSString::UTF8String(nsscreen.localizedName())).to_str().unwrap().to_owned() };
            monitors.push(Monitor {
                id: i as u32,
                name,
                width: nsrect.size.width as u32,
                height: nsrect.size.height as u32,
            });
        };
        Ok(monitors)
    }

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
        let windows_array = unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID) };
        if windows_array.is_null() {
            return Err(Error::CouldNotGetWindowArray);
        }
        let count = unsafe { CFArrayGetCount(windows_array) };
        let mut windows = Vec::with_capacity(count as usize);
        for i in 0..count {
            let window = unsafe { CFArrayGetValueAtIndex(windows_array, i) as CFDictionaryRef };
            if window.is_null() {
                continue;
            }
            let name = unsafe {
                let mut value: *const c_void = std::ptr::null();
                CFDictionaryGetValueIfPresent(window, kCGWindowName as *mut c_void, &mut value);
                value
            };
            if name.is_null() {
                continue;
            }
            let name = match MacosApi::cgstring_to_string(name as CFStringRef) {
                None => {
                    continue;
                }
                Some(name) => {
                    name
                }
            };
            let mut window_bounds: *const c_void = std::ptr::null();
            unsafe { CFDictionaryGetValueIfPresent(window, kCGWindowBounds as *mut c_void, &mut window_bounds) };
            if window_bounds.is_null() {
                continue;
            }
            let window_bounds = unsafe { CFDictionary::from_void(window_bounds) };
            let rect = match CGRect::from_dict_representation(&window_bounds.deref()) {
                None => { continue; }
                Some(rect) => { rect }
            };
            windows.push(Window {
                id: i as u32,
                name,
                width: rect.size.width as u32,
                height: rect.size.height as u32,
            });
        };
        Ok(windows)
    }

    fn capture_display_frame(&self, display: &Monitor) -> Result<Frame, Self::Error> {
        unimplemented!()
    }

    fn update_display_frame(&self, display: &Monitor, cap: &mut Frame) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn capture_window_frame(&self, display: &Window) -> Result<Frame, Self::Error> {
        unimplemented!()
    }

    fn update_window_frame(&self, window: &Window, cap: &mut Frame) -> Result<(), Self::Error> {
        unimplemented!()
    }
}


#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not get Window Array")]
    CouldNotGetWindowArray,
}