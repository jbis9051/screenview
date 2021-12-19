use image::{RgbImage};
use std::error::Error;

struct Display {
    id: u8,
    name: str,
    width: u32,
    height: u32,
}

struct Window {
    id: u8,
    name: str,
    width: u32,
    height: u32,
}

struct MousePosition {
    x: u32,
    y: u32
}

type MouseButtonMask = u8;

struct MouseScroll {
    x: u32,
    y: u32,
}

type Key = u32; // keysym

type ClipboardType = str;

type Frame = RgbImage;

pub(crate) trait NativeAPI: Sized {
    type Error: Error;

    fn new() -> Result<Self, Self::Error>;

    fn key_toggle(&self, key: &Key, down: &bool);

    fn pointer_position(&self) -> Result<MousePosition, Self::Error>;

    fn set_pointer_position(&mut self) -> Result<(), Self::Error>;

    fn click_mouse(&mut self, button_mask: &MouseButtonMask) -> Result<(), Self::Error>;

    fn scroll_mouse(&mut self, scroll: &MouseScroll) -> Result<(), Self::Error>;

    fn clipboard_types(&self) -> Result<(Vec<ClipboardType>), Self::Error>;

    fn clipboard_content(&self, type_name: &ClipboardType) -> Result<([u8]), Self::Error>;

    fn set_clipboard_content(&mut self, type_name: &ClipboardType, content: [u8]) -> Result<(), Self::Error>;

    fn displays(&mut self) -> Result<(Vec<Display>), Self::Error>;

    fn windows(&mut self) -> Result<(Vec<Window>), Self::Error>;

    fn capture_display_frame(&self, display: &Display) -> Result<Frame, Self::Error>;

    fn update_display_frame(&self, display: &Display, cap: &mut Frame) -> Result<(), Self::Error> {
        *cap = self.capture_display_frame(display)?;
        Ok(())
    }

    fn capture_window_frame(&self, display: &Window) -> Result<Frame, Self::Error>;

    fn update_window_frame(&self, window: &Window, cap: &mut Frame) -> Result<(), Self::Error> {
        *cap = self.capture_window_frame(window)?;
        Ok(())
    }

}
