use std::{
    error::Error,
    fmt::{Display, Formatter}, convert::Infallible,
};

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

#[derive(Clone, Debug)]
pub enum ClipboardType {
    Text,
    Custom(String), // TODO other variants will be added later
}

impl Display for ClipboardType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ClipboardType::Text => "ClipboardType::Text",
            ClipboardType::Custom(custom) => return write!(f, "ClipboardType::Custom({})", custom),
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

    fn clipboard_content(&self, type_name: &ClipboardType) -> Result<Vec<u8>, Self::Error>;

    fn set_clipboard_content(
        &mut self,
        type_name: &ClipboardType,
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

#[cfg(dummy_api)]
pub(crate) mod dummy {
    use super::*;

    pub enum DummyApi {}

    impl neon::prelude::Finalize for DummyApi {}

    impl NativeApiTemplate for DummyApi {
        type Error = Infallible;

        fn new() -> Result<Self, Self::Error> {
            unimplemented!()
        }

        fn key_toggle(&mut self, _key: Key, _down: bool) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn pointer_position(&self) -> Result<MousePosition, Self::Error> {
            unimplemented!()
        }

        fn set_pointer_position(&self, _pos: MousePosition) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn toggle_mouse(&self, _button: MouseButton, _down: bool) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn clipboard_content(&self, _type_name: &ClipboardType) -> Result<Vec<u8>, Self::Error> {
            unimplemented!()
        }

        fn set_clipboard_content(
            &mut self,
            _type_name: &ClipboardType,
            _content: &[u8],
        ) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
            unimplemented!()
        }

        fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
            unimplemented!()
        }

        fn capture_display_frame(&self, _display: &Monitor) -> Result<Frame, Self::Error> {
            unimplemented!()
        }

        fn capture_window_frame(&self, _display: &Window) -> Result<Frame, Self::Error> {
            unimplemented!()
        }
    }
}
