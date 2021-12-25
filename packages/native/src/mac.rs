use std::{mem, ptr};
use std::borrow::Borrow;
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::ptr::{null, null_mut};

use cocoa::appkit::*;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSString};
use core_foundation::base::{Boolean, FromVoid, TCFType, ToVoid};
use core_foundation::string::{CFString, CFStringGetCStringPtr, CFStringRef, kCFStringEncodingUTF8};
use core_graphics::display::{CFArray, CFArrayGetCount, CFArrayGetValueAtIndex, CFDictionary, CFDictionaryGetValueIfPresent, CFDictionaryRef, CGRect, kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventRef, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::window::{CGWindowListCopyWindowInfo, kCGWindowBounds, kCGWindowListOptionExcludeDesktopElements, kCGWindowName, kCGWindowOwnerName};
use libc::c_void;
use neon::prelude::Finalize;
use neon::types::StringOverflow;

use crate::api::*;
use crate::keymaps::keycode_mac::KeyCodeMac;
use crate::keymaps::keysym::*;
use crate::keymaps::keysym_to_mac::*;

pub struct MacApi {
    modifier_keys: CGEventFlags,
}

impl Finalize for MacApi {}

impl MacApi {
    fn cgstring_to_string(cf_ref: CFStringRef) -> Option<String> {
        let c_ptr = unsafe { CFStringGetCStringPtr(cf_ref, kCFStringEncodingUTF8) };
        if c_ptr.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(c_ptr).to_str().unwrap().to_owned() })
    }

    fn handle_modifier(&mut self, key_sym: Key, down: bool) -> bool {
        match key_sym {
            XK_Shift_L | XK_Shift_R => {
                if down {
                    self.modifier_keys |= CGEventFlags::CGEventFlagShift;
                } else {
                    self.modifier_keys &= !CGEventFlags::CGEventFlagShift;
                }
            }
            XK_Control_L | XK_Control_R => {
                if down {
                    self.modifier_keys |= CGEventFlags::CGEventFlagControl;
                } else {
                    self.modifier_keys &= !CGEventFlags::CGEventFlagControl;
                }
            }
            XK_Meta_L | XK_Meta_R => {
                if down {
                    self.modifier_keys |= CGEventFlags::CGEventFlagAlternate;
                } else {
                    self.modifier_keys &= !CGEventFlags::CGEventFlagAlternate;
                }
            }
            XK_Alt_L | XK_Alt_R => {
                if down {
                    self.modifier_keys |= CGEventFlags::CGEventFlagCommand;
                } else {
                    self.modifier_keys &= !CGEventFlags::CGEventFlagCommand;
                }
            }
            _ => {
                return false;
            }
        };
        return true;
    }
}

impl NativeApiTemplate for MacApi {
    type Error = Error;

    fn new() -> Result<Self, Self::Error> {
        Ok(Self { modifier_keys: CGEventFlags::empty() })
    }

    fn key_toggle(&mut self, key: Key, down: bool) -> Result<(), Self::Error> {
        self.handle_modifier(key, down);
        let key_code = KEYSYM_MAC.get(&key).unwrap();
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
        let key_event = CGEvent::new_keyboard_event(source, key_code.clone() as CGKeyCode, down).unwrap();
        key_event.set_flags(self.modifier_keys);
        key_event.post(CGEventTapLocation::Session);
        Ok(())
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
            let name = match MacApi::cgstring_to_string(name as CFStringRef) {
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