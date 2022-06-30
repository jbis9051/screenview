use accessibility_sys::{
    kAXErrorSuccess,
    kAXRaiseAction,
    kAXTrustedCheckOptionPrompt,
    kAXWindowsAttribute,
    AXError,
    AXIsProcessTrustedWithOptions,
    AXUIElementCopyAttributeValue,
    AXUIElementCreateApplication,
    AXUIElementPerformAction,
    AXUIElementRef,
};
use std::{
    ffi::CStr,
    fmt::{Display, Formatter},
    ops::Deref,
    os::raw::c_uchar,
    ptr,
    ptr::copy_nonoverlapping,
    slice::from_raw_parts,
};

use block::ConcreteBlock;
use cocoa::{
    appkit::{
        NSApp,
        NSApplicationActivateIgnoringOtherApps,
        NSColorSpace,
        NSEvent,
        NSPasteboard,
        NSPasteboardTypeString,
        NSRunningApplication,
        NSScreen,
    },
    base::{id, nil},
    foundation::{
        NSArray,
        NSData,
        NSDate,
        NSDefaultRunLoopMode,
        NSDictionary,
        NSPoint,
        NSRect,
        NSRunLoop,
        NSString,
        NSUInteger,
    },
};
use core_foundation::{
    base::{CFTypeRef, FromMutVoid, FromVoid, TCFType, TCFTypeRef},
    boolean::CFBoolean,
    number::{kCFNumberIntType, CFNumberGetValue, CFNumberRef},
    string::{CFString, CFStringRef},
};
use core_graphics::{
    display::{
        kCGNullWindowID,
        kCGWindowImageBoundsIgnoreFraming,
        kCGWindowListExcludeDesktopElements,
        kCGWindowListOptionIncludingWindow,
        kCGWindowListOptionOnScreenOnly,
        CFArrayGetCount,
        CFArrayGetValueAtIndex,
        CFDictionary,
        CFDictionaryGetValueIfPresent,
        CFDictionaryRef,
        CGDisplay,
        CGMainDisplayID,
        CGRect,
        CGRectNull,
    },
    event::{
        CGEvent,
        CGEventFlags,
        CGEventTapLocation,
        CGEventType,
        CGKeyCode,
        CGMouseButton,
        ScrollEventUnit,
    },
    event_source::{CGEventSource, CGEventSourceStateID},
    image::CGImage,
    window::{
        kCGWindowBounds,
        kCGWindowLayer,
        kCGWindowName,
        kCGWindowNumber,
        kCGWindowOwnerName,
        kCGWindowOwnerPID,
        CGWindowID,
        CGWindowListCopyWindowInfo,
    },
};
use core_graphics_types::{base::CGFloat, geometry::CGPoint};
use libc::{c_void, pid_t};
use objc::{
    runtime::{BOOL, NO, YES},
    *,
};

use crate::{
    api::*,
    keymaps::{keysym::*, keysym_to_mac::*},
    mac::Error::*,
};

struct MacMonitor {
    id: u32,
    name: String,
    rect: NSRect,
}

#[derive(Debug)]
struct MacWindow {
    id: u32,
    name: String,
    rect: CGRect,
    owner_pid: pid_t,
}

struct MacMousePosition {
    point: NSPoint,
    monitor: MacMonitor,
}

pub struct MacApi {
    modifier_keys: CGEventFlags,
    _nsapplication: id,
}

unsafe impl Send for MacApi {} // TODO make it thread-safe

extern "C" {
    fn NSMouseInRect(aPoint: NSPoint, aRect: NSRect, flipped: BOOL) -> BOOL;
    fn _AXUIElementGetWindow(element: AXUIElementRef, window_id: *mut CGWindowID) -> AXError;
    fn CGPreflightScreenCaptureAccess() -> BOOL;
    fn CGRequestScreenCaptureAccess() -> BOOL;
}

impl From<MacWindow> for Window {
    fn from(w: MacWindow) -> Self {
        Self {
            id: w.id,
            name: w.name,
            width: w.rect.size.width as u32,
            height: w.rect.size.height as u32,
        }
    }
}


impl From<MacMonitor> for Monitor {
    fn from(m: MacMonitor) -> Self {
        Self {
            id: m.id,
            name: m.name,
            width: m.rect.size.width as u32,
            height: m.rect.size.height as u32,
        }
    }
}

impl MacApi {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            modifier_keys: CGEventFlags::empty(),
            _nsapplication: unsafe { NSApp() },
        })
    }

    /// runs an AppKit loop once, fixing caching issues with getting monitors
    fn run_loop() {
        let run_loop: id = unsafe { NSRunLoop::currentRunLoop() };

        let _: id = unsafe {
            msg_send![run_loop,
                runMode: NSDefaultRunLoopMode
                beforeDate: NSDate::distantPast(nil)
            ]
        };
    }

    fn cgstring_to_string(cf_ref: CFStringRef) -> String {
        unsafe { CFString::wrap_under_create_rule(cf_ref) }.to_string()
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
            XK_Shift_L | XK_Shift_R =>
                if down {
                    self.modifier_keys |= CGEventFlags::CGEventFlagShift;
                } else {
                    self.modifier_keys &= !CGEventFlags::CGEventFlagShift;
                },
            XK_Control_L | XK_Control_R =>
                if down {
                    self.modifier_keys |= CGEventFlags::CGEventFlagControl;
                } else {
                    self.modifier_keys &= !CGEventFlags::CGEventFlagControl;
                },
            XK_Meta_L | XK_Meta_R =>
                if down {
                    self.modifier_keys |= CGEventFlags::CGEventFlagAlternate;
                } else {
                    self.modifier_keys &= !CGEventFlags::CGEventFlagAlternate;
                },
            XK_Alt_L | XK_Alt_R =>
                if down {
                    self.modifier_keys |= CGEventFlags::CGEventFlagCommand;
                } else {
                    self.modifier_keys &= !CGEventFlags::CGEventFlagCommand;
                },
            _ => {
                return false;
            }
        };
        true
    }

    fn cgimage_to_frame(image: &CGImage) -> Result<BGRAFrame, ()> {
        let bytes_per_pixel = image.bits_per_pixel() / 8;

        if bytes_per_pixel != 4 {
            return Err(());
        }

        let num_pixels = image.width() * image.height();

        let data = image.data();
        let bgra_padded = data.bytes();

        let mut bgra = Vec::with_capacity(num_pixels * 4);

        let mut bgra_padded_ptr = bgra_padded.as_ptr();
        let mut bgra_ptr = bgra.as_mut_ptr();

        let padding_per_row =
            image.bytes_per_row() - (image.width() * (image.bits_per_pixel() / 8));

        let width = image.width();
        unsafe {
            for i in 0 .. num_pixels {
                copy_nonoverlapping(bgra_padded_ptr, bgra_ptr, 4);
                bgra_padded_ptr = bgra_padded_ptr.add(bytes_per_pixel);
                if i > 0 && i % width == 0 {
                    bgra_padded_ptr = bgra_padded_ptr.add(padding_per_row);
                }
                bgra_ptr = bgra_ptr.add(4);
            }
        }

        unsafe { bgra.set_len((num_pixels * 4) as usize) };

        Ok(BGRAFrame {
            data: bgra,
            width: width as _,
            height: image.height() as _,
        })
    }

    fn set_clipboard_content_impl(type_name: id, content: &[u8]) -> Result<(), Error> {
        let paste_board = unsafe { NSPasteboard::generalPasteboard(nil) };
        unsafe { NSPasteboard::clearContents(paste_board) };
        let data = unsafe {
            NSData::dataWithBytes_length_(
                nil,
                content.as_ptr() as *const c_void,
                content.len() as NSUInteger,
            )
        };
        if unsafe { NSPasteboard::setData_forType(paste_board, data, type_name) == YES } {
            return Ok(());
        }
        Err(ClipboardSetError("Generic".to_string()))
    }

    fn monitors_impl() -> Result<Vec<MacMonitor>, Error> {
        Self::run_loop();

        let display = unsafe { NSScreen::screens(nil) };
        let count = unsafe { NSArray::count(display) };
        let mut monitors = Vec::with_capacity(count as usize);
        for i in 0 .. count {
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
            monitors.push(MacMonitor {
                id: number,
                name,
                rect: nsrect,
            });
        }
        Ok(monitors)
    }

    fn cgnumber_to<T: Default>(number: *const libc::c_void) -> Result<T, ()> {
        let mut value: T = T::default();
        if unsafe {
            CFNumberGetValue(
                number as CFNumberRef,
                kCFNumberIntType,
                (&mut value) as *mut _ as *mut c_void,
            )
        } {
            return Ok(value);
        }
        Err(())
    }

    fn windows_impl() -> Result<Vec<MacWindow>, Error> {
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
        for i in 0 .. count {
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
            let window_id = match Self::cgnumber_to::<u32>(window_id) {
                Ok(num) => num,
                Err(_) => continue,
            };


            let mut owner_pid: *const c_void = ptr::null();
            if unsafe {
                CFDictionaryGetValueIfPresent(
                    window,
                    kCGWindowOwnerPID as *mut c_void,
                    &mut owner_pid,
                )
            } == 0
            {
                continue;
            }
            if owner_pid.is_null() {
                continue;
            }
            let owner_pid = match Self::cgnumber_to::<pid_t>(owner_pid) {
                Ok(num) => num,
                Err(_) => continue,
            } as pid_t;


            let mut window_layer: *const c_void = std::ptr::null();
            if unsafe {
                CFDictionaryGetValueIfPresent(
                    window,
                    kCGWindowLayer as *mut c_void,
                    &mut window_layer,
                )
            } == 0
            {
                continue;
            }
            if window_layer.is_null() {
                continue;
            }

            let window_layer = match Self::cgnumber_to::<u32>(window_layer) {
                Ok(num) => num,
                Err(_) => continue,
            };

            if window_layer != 0 {
                continue;
            }


            let mut name: *const c_void = std::ptr::null();
            if unsafe {
                CFDictionaryGetValueIfPresent(window, kCGWindowName as *mut c_void, &mut name)
            } == 0
            {
                continue;
            }
            if name.is_null()
                && unsafe {
                    CFDictionaryGetValueIfPresent(
                        window,
                        kCGWindowOwnerName as *mut c_void,
                        &mut name,
                    )
                } == 0
            {
                continue;
            }
            if name.is_null() {
                continue;
            }
            let name = Self::cgstring_to_string(name as CFStringRef);

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
            windows.push(MacWindow {
                id: window_id,
                name,
                rect,
                owner_pid,
            });
        }
        Ok(windows)
    }

    fn pointer_position_impl() -> Result<MacMousePosition, Error> {
        let nspoint = unsafe { NSEvent::mouseLocation(nil) };
        let monitor = Self::monitors_impl()?
            .into_iter()
            .find(|m| unsafe { NSMouseInRect(nspoint, m.rect, NO) } == YES)
            .ok_or(Error::MonitorNotFound)?;

        Ok(MacMousePosition {
            point: nspoint,
            monitor,
        })
    }

    fn set_pointer_position_absolute_impl(point: CGPoint) -> Result<(), Error> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| UnableToCreateCGSource)?;
        let event =
            CGEvent::new_mouse_event(source, CGEventType::MouseMoved, point, CGMouseButton::Left)
                .map_err(|_| CGEventError)?;
        event.post(CGEventTapLocation::Session);
        Ok(())
    }

    fn focus_window(window: &MacWindow) -> Result<(), Error> {
        let window_owner = unsafe { AXUIElementCreateApplication(window.owner_pid) };
        let mut windows_ref: CFTypeRef = ptr::null();
        if unsafe {
            let x = AXUIElementCopyAttributeValue(
                window_owner,
                CFString::new(kAXWindowsAttribute).as_concrete_TypeRef(),
                &mut windows_ref as *mut CFTypeRef,
            );
            x
        } != kAXErrorSuccess
        {
            return Err(Error::CouldNotGetWindowsAccessibility);
        }

        if windows_ref.is_null() {
            return Err(Error::CouldNotGetWindowsAccessibility);
        }

        let applications_windows_nsarray = windows_ref as id;

        let target_window_ax = {
            let count = unsafe { NSArray::count(applications_windows_nsarray) };
            let mut window_ax_option: Option<id> = None;
            for i in 0 .. count {
                let window_ax = unsafe { NSArray::objectAtIndex(applications_windows_nsarray, i) };

                let window_id = {
                    let mut window_id: CGWindowID = 0;
                    if unsafe { _AXUIElementGetWindow(window_ax as AXUIElementRef, &mut window_id) }
                        != kAXErrorSuccess
                    {
                        continue;
                    }
                    window_id
                };

                if window_id == window.id {
                    window_ax_option = Some(window_ax);
                }
            }
            window_ax_option
        }
        .ok_or(Error::WindowNotFound(window.id))? as AXUIElementRef;

        if unsafe {
            AXUIElementPerformAction(
                target_window_ax,
                CFString::new(kAXRaiseAction).as_concrete_TypeRef(),
            )
        } != kAXErrorSuccess
        {
            return Err(Error::CouldNotRaiseWindow);
        }

        Ok(())
    }

    fn activate_window(window: &MacWindow) -> Result<(), Error> {
        // TODO: remove when publishes this
        let app: id = unsafe {
            msg_send![class!(NSRunningApplication), runningApplicationWithProcessIdentifier:window.owner_pid]
        };
        if unsafe {
            NSRunningApplication::activateWithOptions_(app, NSApplicationActivateIgnoringOtherApps)
                == YES
        } {
            Ok(())
        } else {
            Err(Error::CouldNotActivateApplication)
        }
    }

    fn focus_and_activate_window(window: &MacWindow) -> Result<(), Error> {
        Self::activate_window(window)?;
        Self::focus_window(window)?;
        Ok(())
    }

    fn capture_monitor_frame_impl(monitor_id: MonitorId) -> Result<BGRAFrame, Error> {
        let core_display = CGDisplay::new(monitor_id);
        let frame = core_display
            .image()
            .ok_or_else(|| CaptureDisplayError(monitor_id.to_string()))?;
        Self::cgimage_to_frame(&frame).map_err(|_| CaptureDisplayError(monitor_id.to_string()))
    }

    fn capture_window_frame_impl(window_id: WindowId) -> Result<BGRAFrame, Error> {
        let image = CGDisplay::screenshot(
            unsafe { CGRectNull },
            kCGWindowListOptionIncludingWindow,
            window_id,
            kCGWindowImageBoundsIgnoreFraming,
        )
        .ok_or_else(|| CaptureDisplayError(window_id.to_string()))?;
        Self::cgimage_to_frame(&image).map_err(|_| CaptureDisplayError(window_id.to_string()))
    }

    pub fn accessibility_permission(prompt: bool) -> bool {
        let key =
            unsafe { CFString::wrap_under_create_rule(kAXTrustedCheckOptionPrompt) }.as_CFType();
        let value = CFBoolean::from(prompt).as_CFType();
        let dict = CFDictionary::from_CFType_pairs(&[(key, value)]);
        unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) }
    }

    pub fn screen_capture_permission() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() == YES }
    }

    pub fn screen_capture_permission_prompt() -> bool {
        unsafe { CGRequestScreenCaptureAccess() == YES }
    }
}

impl NativeApiTemplate for MacApi {
    type Error = Error;

    fn key_toggle(&mut self, key: Key, down: bool) -> Result<(), Error> {
        self.handle_modifier(key, down);
        let key_code = KEYSYM_MAC.get(&key).ok_or(KeyNotFoundError(key))?;
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| UnableToCreateCGSource)?;
        let key_event = CGEvent::new_keyboard_event(source, *key_code as CGKeyCode, down)
            .map_err(|_| CGEventError)?;
        key_event.set_flags(self.modifier_keys);
        key_event.post(CGEventTapLocation::Session);
        Ok(())
    }

    fn pointer_position(&mut self, windows: &[WindowId]) -> Result<MousePosition, Error> {
        let position = Self::pointer_position_impl()?;
        let normalized_position = NSPoint::new(
            position.point.x - position.monitor.rect.origin.x,
            position.monitor.rect.size.height - position.point.y - position.monitor.rect.origin.y,
        );

        let windows: Vec<MacWindow> = Self::windows_impl()?
            .into_iter()
            .filter(|w| unsafe {
                let nsrect: NSRect = std::mem::transmute(w.rect);
                NSMouseInRect(normalized_position, nsrect, NO) == YES
            })
            // if we ever decide to you know...not cheat https://developer.apple.com/documentation/coregraphics/1455215-cgwindowlistcreatedescriptionfro
            // Put the O(n^2) filter after the other filter under the assumption that the n^2
            // would be more expensive otherwise
            .filter(|w| windows.iter().any(|&id| w.id == id))
            .collect();

        Ok(MousePosition {
            x: normalized_position.x as u32,
            y: normalized_position.y as u32,
            monitor_id: position.monitor.id,
            window_relatives: windows
                .iter()
                .map(|w| PointerPositionRelative {
                    x: (normalized_position.x - w.rect.origin.x) as u32,
                    y: (normalized_position.y - w.rect.origin.y) as u32,
                    window_id: w.id,
                })
                .collect(),
        })
    }

    fn set_pointer_position_absolute(
        &mut self,
        x: u32,
        y: u32,
        monitor_id: MonitorId,
    ) -> Result<(), Self::Error> {
        let monitor = Self::monitors_impl()?
            .into_iter()
            .find(|m| m.id == monitor_id as u32)
            .ok_or(MonitorNotFound)?;
        let point = CGPoint::new(
            x as CGFloat + monitor.rect.origin.x,
            y as CGFloat + monitor.rect.origin.y,
        ); // CGEvent uses origin of upper left I guess
        Self::set_pointer_position_absolute_impl(point)?;
        Ok(())
    }

    fn set_pointer_position_relative(
        &mut self,
        x: u32,
        y: u32,
        window_id: WindowId,
    ) -> Result<(), Self::Error> {
        let window = Self::windows_impl()?
            .into_iter()
            .find(|w| w.id == window_id)
            .ok_or(Error::WindowNotFound(window_id))?;
        let absolute_mouse = CGPoint::new(
            window.rect.origin.x + x as f64,
            window.rect.origin.y + y as f64,
        );
        Self::set_pointer_position_absolute_impl(absolute_mouse)
    }

    fn toggle_mouse(
        &mut self,
        button: MouseButton,
        down: bool,
        window_id: Option<WindowId>,
    ) -> Result<(), Error> {
        if let Some(window_id) = window_id {
            let window = Self::windows_impl()?
                .into_iter()
                .find(|w| w.id == window_id)
                .ok_or(Error::WindowNotFound(window_id))?;
            Self::focus_and_activate_window(&window)?;
        }

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
                let mouse_position = Self::pointer_position_impl()?;
                let mouse_position = CGPoint::new(
                    mouse_position.point.x as CGFloat,
                    mouse_position.point.y as CGFloat,
                );
                let mouse_type = match button {
                    MouseButton::Left =>
                        if down {
                            CGEventType::LeftMouseDown
                        } else {
                            CGEventType::LeftMouseUp
                        },
                    MouseButton::Right =>
                        if down {
                            CGEventType::RightMouseDown
                        } else {
                            CGEventType::RightMouseUp
                        },
                    _ =>
                        if down {
                            CGEventType::OtherMouseDown
                        } else {
                            CGEventType::OtherMouseUp
                        },
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

    fn clipboard_content(&mut self, type_name: &ClipboardType) -> Result<Option<Vec<u8>>, Error> {
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
            return Ok(None);
        }
        Ok(Some(Self::nsdata_to_vec(data)))
    }

    fn set_clipboard_content(
        &mut self,
        type_name: &ClipboardType,
        content: &[u8],
    ) -> Result<(), Error> {
        let type_name = unsafe {
            match type_name {
                ClipboardType::Text => NSPasteboardTypeString,
                ClipboardType::Custom(type_name) =>
                    NSString::alloc(nil).init_str(type_name.as_str()),
            }
        };
        Self::set_clipboard_content_impl(type_name, content)
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Error> {
        Ok(Self::monitors_impl()?
            .into_iter()
            .map(|m| m.into())
            .collect())
    }

    fn windows(&mut self) -> Result<Vec<Window>, Error> {
        Ok(Self::windows_impl()?
            .into_iter()
            .map(|w| w.into())
            .collect())
    }

    fn capture_monitor_frame(&mut self, monitor_id: MonitorId) -> Result<BGRAFrame, Error> {
        Self::capture_monitor_frame_impl(monitor_id)
    }

    fn capture_window_frame(&mut self, window_id: WindowId) -> Result<BGRAFrame, Error> {
        Self::capture_window_frame_impl(window_id)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not get Window Array")]
    CouldNotGetWindowArray,
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
    #[error("Couldn't find a matching monitor with the mouse position")]
    MonitorNotFound,
    #[error("Couldn't find a window for the id {0}")]
    WindowNotFound(WindowId),
    #[error("Couldn't get windows from AXUIElementCopyAttributeValue")]
    CouldNotGetWindowsAccessibility,
    #[error("Could not raise window AXUIElementPerformAction")]
    CouldNotRaiseWindow,
    #[error("Could not activate")]
    CouldNotActivateApplication,
}
