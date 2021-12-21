use image::{RgbImage};
use std::error::Error;

pub struct Monitor {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

pub struct Window {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy)]
pub struct MousePosition {
    pub x: u32,
    pub y: u32
}

#[derive(Clone, Copy)]
pub enum MouseButton {
    Left,
    Center,
    Right,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    Button4,
    Button5
}

#[derive(Clone, Copy)]
pub struct MouseScroll {
    pub x: i32,
    pub y: i32,
}

pub type Key = u32; // keysym

pub enum ClipboardType {
    Text,
    // Other variants will be added later
}

pub type Frame = RgbImage;

pub(crate) trait NativeAPI: Sized {
    type Error: Error;

    fn new() -> Result<Self, Self::Error>;

    fn key_toggle(&self, key: Key, down: bool);

    fn pointer_position(&self) -> Result<MousePosition, Self::Error>;

    fn set_pointer_position(&self, pos: MousePosition) -> Result<(), Self::Error>;

    fn toggle_mouse(&self, button: MouseButton, down: bool) -> Result<(), Self::Error>;

    fn scroll_mouse(&self, scroll: MouseScroll) -> Result<(), Self::Error>;

    fn clipboard_types(&self) -> Result<Vec<ClipboardType>, Self::Error>;

    fn clipboard_content(&self, type_name: &ClipboardType) -> Result<Vec<u8>, Self::Error>;

    fn set_clipboard_content(&mut self, type_name: &ClipboardType, content: &[u8]) -> Result<(), Self::Error>;

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error>;

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error>;

    fn capture_display_frame(&self, display: &Monitor) -> Result<Frame, Self::Error>;

    fn update_display_frame(&self, display: &Monitor, cap: &mut Frame) -> Result<(), Self::Error> {
        *cap = self.capture_display_frame(display)?;
        Ok(())
    }

    fn capture_window_frame(&self, display: &Window) -> Result<Frame, Self::Error>;

    fn update_window_frame(&self, window: &Window, cap: &mut Frame) -> Result<(), Self::Error> {
        *cap = self.capture_window_frame(window)?;
        Ok(())
    }
}
