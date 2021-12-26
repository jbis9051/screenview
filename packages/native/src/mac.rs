use std::ffi::CStr;
use std::ops::Deref;
use std::os::raw::c_uchar;
use std::ptr;
use std::slice::from_raw_parts;

use cocoa::appkit::{NSColorSpace, NSPasteboard, NSPasteboardTypeString, NSScreen};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSData, NSDictionary, NSString, NSUInteger};
use core_foundation::base::FromVoid;
use core_foundation::number::{CFNumber, CFNumberGetValue, CFNumberRef, CFNumberType, kCFNumberIntType};
use core_foundation::string::{kCFStringEncodingUTF8, CFStringGetCStringPtr, CFStringRef};
use core_graphics::display::{kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly, CFArrayGetCount, CFArrayGetValueAtIndex, CFDictionary, CFDictionaryGetValueIfPresent, CFDictionaryRef, CGRect, CGDisplayCreateImage, CGDisplay, CGMainDisplayID, CGRectNull, kCGWindowListOptionIncludingWindow, kCGWindowImageBoundsIgnoreFraming};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGKeyCode, CGMouseButton,
    ScrollEventUnit,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::image::{CGImage, CGImageRef};
use core_graphics::window::{kCGWindowBounds, kCGWindowName, CGWindowListCopyWindowInfo, kCGWindowNumber, CGWindowListCreateImage};
use core_graphics_types::base::CGFloat;
use core_graphics_types::geometry::{CGPoint, CGSize};
use image::DynamicImage::ImageRgba8;
use image::{Bgra, DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage, RgbImage};
use image::ImageFormat::{Bmp, Png};
use libc::{c_void, intptr_t};
use neon::prelude::Finalize;

use crate::api::*;
use crate::keymaps::keysym::*;
use crate::keymaps::keysym_to_mac::*;
use crate::mac::Error::{CaptureDisplayError, ClipboardNotFound, ClipboardSetError};

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

    fn nsdata_to_vec(data: id) -> Vec<u8> {
        let length = unsafe { NSData::length(data) } as usize;
        let ptr = unsafe { data.bytes() } as *const c_uchar;
        if ptr.is_null() {
            return Vec::new();
        }
        unsafe { from_raw_parts(ptr, length) }.to_vec()
    }

    #[allow(non_upper_case_globals)]
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
        true
    }

    fn set_clipboard_content_mac(type_name: id, content: &[u8]) -> Result<(), Error> {
        let paste_board = unsafe { NSPasteboard::generalPasteboard(nil) };
        unsafe { NSPasteboard::clearContents(paste_board) };
        let data = unsafe {
            NSData::dataWithBytes_length_(
                nil,
                content.as_ptr() as *const c_void,
                content.len() as NSUInteger,
            )
        };
        if unsafe { NSPasteboard::setData_forType(paste_board, data, type_name) } {
            return Ok(());
        }
        Err(ClipboardSetError("Generic".to_string()))
    }

    fn cgimage_to_frame(image: &CGImage) -> Result<Frame, ()> {
        let bytes_per_pixel = (image.bits_per_pixel() / 8);
        if bytes_per_pixel != 4 {
            // TODO error
        }
        let data = image.data();
        let rgba = data.bytes();
        let rgb = vec![0u8; image.width() * image.height() * 3];
        let mut rgba_ptr = rgba.as_ptr();
        let mut rgb_ptr = rgb.as_ptr();
        let num_pixels = image.width() * image.height();
        let padding_per_row = image.bytes_per_row() - (image.width() * (image.bits_per_pixel() / 8));
        let width = image.width();
        unsafe {
            for i in 0..num_pixels {
                let [b, g, r] = *(rgba_ptr as *const [u8; 3]);
                *(rgb_ptr as *mut [u8; 3]) = [r, g, b];
                rgba_ptr = rgba_ptr.add(bytes_per_pixel);
                if i > 0 && i % width == 0 {
                    rgba_ptr = rgba_ptr.add(padding_per_row);
                }
                rgb_ptr = rgb_ptr.add(3);
            }
        }
        Ok(RgbImage::from_vec(image.width() as u32, image.height() as u32, rgb).expect("couldn't convert"))
    }
}

impl NativeApiTemplate for MacApi {
    type Error = Error;

    fn new() -> Result<Self, Self::Error> {
        Ok(Self {
            modifier_keys: CGEventFlags::empty(),
        })
    }

    fn key_toggle(&mut self, key: Key, down: bool) -> Result<(), Self::Error> {
        self.handle_modifier(key, down);
        let key_code = KEYSYM_MAC.get(&key).unwrap();
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
        let key_event = CGEvent::new_keyboard_event(source, *key_code as CGKeyCode, down).unwrap();
        key_event.set_flags(self.modifier_keys);
        key_event.post(CGEventTapLocation::Session);
        Ok(())
    }

    fn pointer_position(&self) -> Result<MousePosition, Self::Error> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
        let event = CGEvent::new(source).unwrap();
        let point = event.location();
        Ok(MousePosition {
            x: point.x as u32,
            y: point.y as u32,
            monitor_id: 0,
        })
    }

    fn set_pointer_position(&self, pos: MousePosition) -> Result<(), Self::Error> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
        let event = CGEvent::new_mouse_event(
            source,
            CGEventType::MouseMoved,
            CGPoint::new(pos.x as CGFloat, pos.y as CGFloat),
            CGMouseButton::Left,
        )
            .unwrap();
        event.post(CGEventTapLocation::Session);
        Ok(())
    }

    fn toggle_mouse(&self, button: MouseButton, down: bool) -> Result<(), Self::Error> {
        // TODO can we get smooth scrolling?
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
        match button {
            MouseButton::ScrollUp
            | MouseButton::ScrollDown
            | MouseButton::ScrollLeft
            | MouseButton::ScrollRight => {
                let scroll_speed = 4;
                let scroll_event = match button {
                    MouseButton::ScrollUp => CGEvent::new_scroll_event(
                        source,
                        ScrollEventUnit::PIXEL,
                        2,
                        scroll_speed,
                        0,
                        0,
                    ),
                    MouseButton::ScrollDown => CGEvent::new_scroll_event(
                        source,
                        ScrollEventUnit::PIXEL,
                        2,
                        -scroll_speed,
                        0,
                        0,
                    ),
                    MouseButton::ScrollLeft => CGEvent::new_scroll_event(
                        source,
                        ScrollEventUnit::PIXEL,
                        2,
                        0,
                        -scroll_speed,
                        0,
                    ),
                    MouseButton::ScrollRight => CGEvent::new_scroll_event(
                        source,
                        ScrollEventUnit::PIXEL,
                        2,
                        0,
                        scroll_speed,
                        0,
                    ),
                    _ => Err(()),
                }
                    .unwrap();
                scroll_event.post(CGEventTapLocation::Session);
            }
            _ => {
                let mouse_position = self.pointer_position()?;
                let mouse_position =
                    CGPoint::new(mouse_position.x as CGFloat, mouse_position.y as CGFloat);
                let mouse_type = match button {
                    MouseButton::Left => {
                        if down {
                            CGEventType::LeftMouseDown
                        } else {
                            CGEventType::LeftMouseUp
                        }
                    }
                    MouseButton::Right => {
                        if down {
                            CGEventType::RightMouseDown
                        } else {
                            CGEventType::RightMouseUp
                        }
                    }
                    _ => {
                        if down {
                            CGEventType::OtherMouseDown
                        } else {
                            CGEventType::OtherMouseUp
                        }
                    }
                };
                let moose_button = match button {
                    MouseButton::Left => CGMouseButton::Left,
                    MouseButton::Right => CGMouseButton::Right,
                    _ => CGMouseButton::Center,
                };
                let event =
                    CGEvent::new_mouse_event(source, mouse_type, mouse_position, moose_button)
                        .unwrap();
                event.post(CGEventTapLocation::Session);
            }
        }
        Ok(())
    }

    fn clipboard_content(&self, type_name: ClipboardType) -> Result<Vec<u8>, Self::Error> {
        let paste_board = unsafe { NSPasteboard::generalPasteboard(nil) };
        match type_name {
            ClipboardType::Text => {
                let data =
                    unsafe { NSPasteboard::dataForType(paste_board, NSPasteboardTypeString) };
                if data == nil {
                    return Err(ClipboardNotFound(ClipboardType::Text.to_string()));
                }
                Ok(Self::nsdata_to_vec(data)) // TODO may be null terminated :(
            }
        }
    }

    fn clipboard_content_custom(&self, type_name: &str) -> Result<Vec<u8>, Self::Error> {
        let paste_board = unsafe { NSPasteboard::generalPasteboard(nil) };
        let data = unsafe {
            NSPasteboard::dataForType(paste_board, NSString::alloc(nil).init_str(type_name))
        };
        if data == nil {
            return Err(ClipboardNotFound(type_name.to_string()));
        }
        Ok(Self::nsdata_to_vec(data))
    }

    fn set_clipboard_content(
        &mut self,
        type_name: ClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error> {
        let type_name = unsafe {
            match type_name {
                ClipboardType::Text => NSPasteboardTypeString,
            }
        };
        MacApi::set_clipboard_content_mac(type_name, content)
    }

    fn set_clipboard_content_custom(
        &mut self,
        type_name: &str,
        content: &[u8],
    ) -> Result<(), Self::Error> {
        MacApi::set_clipboard_content_mac(
            unsafe { NSString::alloc(nil).init_str(type_name) },
            content,
        )
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
        let display = unsafe { NSScreen::screens(nil) };
        let count = unsafe { NSArray::count(display) };
        let mut monitors = Vec::with_capacity(count as usize);
        for i in 0..count {
            let nsscreen = unsafe { NSArray::objectAtIndex(display, i) };
            let nsrect = unsafe { NSScreen::frame(nsscreen) };
            let nsdictionary = unsafe { NSScreen::deviceDescription(nsscreen) };
            let nsnumber = unsafe { NSDictionary::objectForKey_(nsdictionary, NSString::alloc(nil).init_str("NSScreenNumber")) };
            let mut number: u32 = 0u32;
            if !unsafe { CFNumberGetValue(nsnumber as CFNumberRef, kCFNumberIntType, (&mut number) as *mut _ as *mut c_void) } {
                continue;
            };
            let name = unsafe {
                CStr::from_ptr(NSString::UTF8String(nsscreen.localizedName()))
                    .to_str()
                    .unwrap()
                    .to_owned()
            };
            monitors.push(Monitor {
                id: number,
                name,
                width: nsrect.size.width as u32,
                height: nsrect.size.height as u32,
            });
        }
        Ok(monitors)
    }

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
        let windows_array = unsafe {
            CGWindowListCopyWindowInfo(
                kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
                kCGNullWindowID,
            )
        };
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
            let mut window_id: *const c_void = std::ptr::null();
            if unsafe {
                CFDictionaryGetValueIfPresent(
                    window,
                    kCGWindowNumber as *mut c_void,
                    &mut window_id,
                )
            } == 0 {
                continue;
            }
            if window_id.is_null() {
                continue;
            }
            let window_id = unsafe {
                let mut number: u32 = 0u32;
                if !unsafe { CFNumberGetValue(window_id as CFNumberRef, kCFNumberIntType, (&mut number) as *mut _ as *mut c_void) } {
                    continue;
                };
                number
            };
            let mut name: *const c_void = std::ptr::null();
            if unsafe { CFDictionaryGetValueIfPresent(window, kCGWindowName as *mut c_void, &mut name) } == 0 {
                continue;
            }
            if name.is_null() {
                continue;
            }
            let name = match MacApi::cgstring_to_string(name as CFStringRef) {
                None => {
                    continue;
                }
                Some(name) => name,
            };
            let mut window_bounds: *const c_void = std::ptr::null();
            if unsafe {
                CFDictionaryGetValueIfPresent(
                    window,
                    kCGWindowBounds as *mut c_void,
                    &mut window_bounds,
                )
            } == 0 {
                continue;
            }
            if window_bounds.is_null() {
                continue;
            }
            let window_bounds = unsafe { CFDictionary::from_void(window_bounds) };
            let rect = match CGRect::from_dict_representation(window_bounds.deref()) {
                None => {
                    continue;
                }
                Some(rect) => rect,
            };
            windows.push(Window {
                id: window_id,
                name,
                width: rect.size.width as u32,
                height: rect.size.height as u32,
            });
        }
        Ok(windows)
    }

    fn capture_display_frame(&self, display: &Monitor) -> Result<Frame, Self::Error> {
        let core_display = CGDisplay::new(display.id);
        let frame = core_display.image().expect("bad image");
        MacApi::cgimage_to_frame(&frame).map_err(|_| CaptureDisplayError(display.name.clone()))
    }

    fn capture_window_frame(&self, window: &Window) -> Result<Frame, Self::Error> {
        //let image = match CGDisplay::screenshot(unsafe { CGRect::new(&CGPoint::new(CGFloat::from(400), CGFloat::from(400)), &CGSize::new(CGFloat::from(1), CGFloat::from(1))) }, kCGWindowListOptionIncludingWindow, window.id, kCGWindowImageBoundsIgnoreFraming) {
        let image = match CGDisplay::screenshot(unsafe { CGRectNull }, kCGWindowListOptionIncludingWindow, window.id, kCGWindowImageBoundsIgnoreFraming) {
            None => {
                return Err(CaptureDisplayError(window.name.clone()));
            }
            Some(img) => { img }
        };
        MacApi::cgimage_to_frame(&image).map_err(|_| CaptureDisplayError(window.name.clone()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not get Window Array")]
    CouldNotGetWindowArray,
    #[error("Could not get pasteboard data for key `{0}`")]
    ClipboardNotFound(String),
    #[error("Could not set pasteboard data for key `{0}`")]
    ClipboardSetError(String),
    #[error("Could not capture display: `{0}`")]
    CaptureDisplayError(String),
    #[error("Could not capture window: `{0}`")]
    CaptureWindowError(String),
}
