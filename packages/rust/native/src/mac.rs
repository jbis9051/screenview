use std::{ffi::CStr, ops::Deref, os::raw::c_uchar, ptr, slice::from_raw_parts, str::Utf8Error};

use cocoa::{
    appkit::{NSColorSpace, NSPasteboard, NSPasteboardTypeString, NSScreen},
    base::{id, nil},
    foundation::{NSArray, NSData, NSDictionary, NSString, NSUInteger},
};
use core_foundation::{
    base::FromVoid,
    number::{kCFNumberIntType, CFNumber, CFNumberGetValue, CFNumberRef, CFNumberType},
    string::{kCFStringEncodingUTF8, CFStringGetCStringPtr, CFStringRef},
};
use core_graphics::{
    display::{
        kCGNullWindowID, kCGWindowImageBoundsIgnoreFraming, kCGWindowListExcludeDesktopElements,
        kCGWindowListOptionIncludingWindow, kCGWindowListOptionOnScreenOnly, CFArrayGetCount,
        CFArrayGetValueAtIndex, CFDictionary, CFDictionaryGetValueIfPresent, CFDictionaryRef,
        CGDisplay, CGDisplayCreateImage, CGMainDisplayID, CGRect, CGRectNull,
    },
    event::{
        CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGKeyCode, CGMouseButton,
        ScrollEventUnit,
    },
    event_source::{CGEventSource, CGEventSourceStateID},
    image::{CGImage, CGImageRef},
    window::{
        kCGWindowBounds, kCGWindowName, kCGWindowNumber, CGWindowListCopyWindowInfo,
        CGWindowListCreateImage,
    },
};
use core_graphics_types::{
    base::CGFloat,
    geometry::{CGPoint, CGSize},
};
use image::{
    Bgra, DynamicImage,
    DynamicImage::ImageRgba8,
    ImageBuffer, ImageFormat,
    ImageFormat::{Bmp, Png},
    RgbImage, Rgba, RgbaImage,
};
use libc::{c_void, intptr_t};
use neon::prelude::Finalize;

use crate::{
    api::*,
    keymaps::{keycode_mac::KeyCodeMac, keysym::*, keysym_to_mac::*},
    mac::Error::*,
};

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
        Some(unsafe { CStr::from_ptr(c_ptr) }.to_str().ok()?.to_owned())
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
        let padding_per_row =
            image.bytes_per_row() - (image.width() * (image.bits_per_pixel() / 8));
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
        Ok(
            RgbImage::from_vec(image.width() as u32, image.height() as u32, rgb)
                .expect("couldn't convert"),
        )
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
        let key_code = KEYSYM_MAC.get(&key).ok_or_else(|| KeyNotFoundError(key))?;
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| UnableToCreateCGSource)?;
        let key_event = CGEvent::new_keyboard_event(source, *key_code as CGKeyCode, down)
            .map_err(|_| CGEventError)?;
        key_event.set_flags(self.modifier_keys);
        key_event.post(CGEventTapLocation::Session);
        Ok(())
    }

    fn pointer_position(&self) -> Result<MousePosition, Self::Error> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| UnableToCreateCGSource)?;
        let event = CGEvent::new(source).map_err(|_| CGEventError)?;
        let point = event.location();
        Ok(MousePosition {
            x: point.x as u32,
            y: point.y as u32,
            monitor_id: 0,
        })
    }

    fn set_pointer_position(&self, pos: MousePosition) -> Result<(), Self::Error> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| UnableToCreateCGSource)?;
        let event = CGEvent::new_mouse_event(
            source,
            CGEventType::MouseMoved,
            CGPoint::new(pos.x as CGFloat, pos.y as CGFloat),
            CGMouseButton::Left,
        )
        .map_err(|_| CGEventError)?;
        event.post(CGEventTapLocation::Session);
        Ok(())
    }

    fn toggle_mouse(&self, button: MouseButton, down: bool) -> Result<(), Self::Error> {
        // TODO can we get smooth scrolling?
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| UnableToCreateCGSource)?;
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
                .map_err(|_| CGEventError)?;
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
                        .map_err(|_| CGEventError)?;
                event.post(CGEventTapLocation::Session);
            }
        }
        Ok(())
    }

    fn clipboard_content(&self, type_name: &ClipboardType) -> Result<Vec<u8>, Self::Error> {
        let paste_board = unsafe { NSPasteboard::generalPasteboard(nil) };
        if paste_board == nil {
            return Err(NSPasteboardError);
        }
        let data = match type_name {
            ClipboardType::Text => unsafe {
                NSPasteboard::dataForType(paste_board, NSPasteboardTypeString)
            },
            ClipboardType::Custom(type_name) => unsafe {
                NSPasteboard::dataForType(
                    paste_board,
                    NSString::alloc(nil).init_str(type_name.as_str()),
                )
            },
        };
        if data == nil {
            return Err(ClipboardNotFound(type_name.to_string()));
        }
        Ok(Self::nsdata_to_vec(data)) // TODO may be null terminated :(
    }

    fn set_clipboard_content(
        &mut self,
        type_name: &ClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error> {
        let type_name = unsafe {
            match type_name {
                ClipboardType::Text => NSPasteboardTypeString,
                ClipboardType::Custom(type_name) => unsafe {
                    NSString::alloc(nil).init_str(type_name.as_str())
                },
            }
        };
        MacApi::set_clipboard_content_mac(type_name, content)
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
        let display = unsafe { NSScreen::screens(nil) };
        let count = unsafe { NSArray::count(display) };
        let mut monitors = Vec::with_capacity(count as usize);
        for i in 0..count {
            let nsscreen = unsafe { NSArray::objectAtIndex(display, i) };
            let nsrect = unsafe { NSScreen::frame(nsscreen) };
            let nsdictionary = unsafe { NSScreen::deviceDescription(nsscreen) };
            let nsnumber = unsafe {
                NSDictionary::objectForKey_(
                    nsdictionary,
                    NSString::alloc(nil).init_str("NSScreenNumber"),
                )
            };
            let mut number: u32 = 0u32;
            if !unsafe {
                CFNumberGetValue(
                    nsnumber as CFNumberRef,
                    kCFNumberIntType,
                    (&mut number) as *mut _ as *mut c_void,
                )
            } {
                continue;
            };
            let name = unsafe {
                CStr::from_ptr(NSString::UTF8String(nsscreen.localizedName()))
                    .to_str()
                    .map_err(|_| NSStringError)?
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
            } == 0
            {
                continue;
            }
            if window_id.is_null() {
                continue;
            }
            let window_id = unsafe {
                let mut number: u32 = 0u32;
                if !unsafe {
                    CFNumberGetValue(
                        window_id as CFNumberRef,
                        kCFNumberIntType,
                        (&mut number) as *mut _ as *mut c_void,
                    )
                } {
                    continue;
                };
                number
            };
            let mut name: *const c_void = std::ptr::null();
            if unsafe {
                CFDictionaryGetValueIfPresent(window, kCGWindowName as *mut c_void, &mut name)
            } == 0
            {
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
            } == 0
            {
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
        let frame = core_display
            .image()
            .ok_or_else(|| CaptureDisplayError(display.name.clone()))?;
        MacApi::cgimage_to_frame(&frame).map_err(|_| CaptureDisplayError(display.name.clone()))
    }

    fn capture_window_frame(&self, window: &Window) -> Result<Frame, Self::Error> {
        let image = CGDisplay::screenshot(
            unsafe { CGRectNull },
            kCGWindowListOptionIncludingWindow,
            window.id,
            kCGWindowImageBoundsIgnoreFraming,
        )
        .ok_or_else(|| CaptureDisplayError(window.name.clone()))?;
        Ok(MacApi::cgimage_to_frame(&image)
            .map_err(|_| CaptureDisplayError(window.name.clone()))?)
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
    #[error("Keymap not found for keysym `{0}`")]
    KeyNotFoundError(u32),
    #[error("Could not create CG Source")]
    UnableToCreateCGSource,
    #[error("Could not create or post CG event")]
    CGEventError,
    #[error("Error occurred with NSPasteboard API")]
    NSPasteboardError,
    #[error("Error occurred with NSString API")]
    NSStringError,
}