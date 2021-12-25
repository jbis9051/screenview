use std::error::Error;
use std::fmt::{Display, Formatter};

use image::RgbImage;

#[derive(Debug)]
pub struct Monitor {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct Window {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct MousePosition {
    pub x: u32,
    pub y: u32,
    pub monitor_id: u32,
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
    Button5,
}

pub type Key = u32; // keysym

#[derive(Clone, Copy, Debug)]
pub enum ClipboardType {
    Text,
    // TODO other variants will be added later
}

impl Display for ClipboardType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ClipboardType::Text => "ClipboardType::Text"
        };
        write!(f, "{}", str)
    }
}

pub type Frame = RgbImage;

pub(crate) trait NativeApiTemplate: Sized {
    type Error: Error;

    fn new() -> Result<Self, Self::Error>;

    fn key_toggle(&mut self, key: Key, down: bool) -> Result<(), Self::Error>;

    fn pointer_position(&self) -> Result<MousePosition, Self::Error>;

    fn set_pointer_position(&self, pos: MousePosition) -> Result<(), Self::Error>;

    fn toggle_mouse(&self, button: MouseButton, down: bool) -> Result<(), Self::Error>;

    fn clipboard_content(&self, type_name: ClipboardType) -> Result<Vec<u8>, Self::Error>;

    fn clipboard_content_custom(&self, type_name: &str) -> Result<Vec<u8>, Self::Error>;

    fn set_clipboard_content(
        &mut self,
        type_name: ClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error>;

    fn set_clipboard_content_custom(
        &mut self,
        type_name: &str,
        content: &[u8],
    ) -> Result<(), Self::Error>;

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
