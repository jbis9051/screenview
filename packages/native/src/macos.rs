use std::ffi::CStr;
use cocoa::appkit::*;
use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use cocoa_foundation::foundation::*;
use core_graphics::display::*;
use core_graphics::window::*;
use neon::prelude::Finalize;
use crate::api::*;

pub struct MacosApi;

impl Finalize for MacosApi {}

impl NativeApiTemplate for MacosApi {
    type Error = std::convert::Infallible;

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
        let mut monitors = vec![];
        for i in 0..count {
            let nsscreen = unsafe { NSArray::objectAtIndex(display, i)};
            let nsrect = unsafe {NSScreen::frame(nsscreen)};
            let name = unsafe {CStr::from_ptr(NSString::UTF8String(nsscreen.localizedName())).to_str().unwrap().to_owned()};
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
        let windowsArray = unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionExcludeDesktopElements, kCGNullWindowID) };
        let count = unsafe { NSArray::count(windows) };
        let mut windows = vec![];
        for i in 0..count {
            let nsdictionary = unsafe { NSArray::objectAtIndex(windowsArray, i)};
            let val = unsafe {NSDictionary::objectForKey_(nsdictionary,kCGWindowName)};
            let name = unsafe {CStr::from_ptr(NSString::UTF8String(nsscreen.localizedName())).to_str().unwrap().to_owned()};
            windows.push(Monitor {
                id: i as u32,
                name,
                width: nsrect.size.width as u32,
                height: nsrect.size.height as u32,
            });
        };
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