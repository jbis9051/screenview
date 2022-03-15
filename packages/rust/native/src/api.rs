use std::fmt::{Debug, Display, Formatter};

use image::RgbImage;

pub type MonitorId = u32;
pub type WindowId = u32;

#[derive(Debug, Clone)]
pub struct Monitor {
    pub id: MonitorId,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointerPositionRelative {
    pub x: u32,
    pub y: u32,
    pub window_id: WindowId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MousePosition {
    pub x: u32,
    pub y: u32,
    pub monitor_id: MonitorId,
    pub window_relatives: Vec<PointerPositionRelative>,
}

#[derive(Clone, Copy, Debug)]
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

pub trait NativeApiTemplate {
    type Error: Debug;

    fn key_toggle(&mut self, key: Key, down: bool) -> Result<(), Self::Error>;

    /// Returns current MousePosition and all Window's the mouse intersect. Intuitively, this should only be one but because Windows can be layered it can be multiple.
    fn pointer_position(&mut self) -> Result<MousePosition, Self::Error>;

    fn set_pointer_position_absolute(
        &mut self,
        x: u32,
        y: u32,
        monitor_id: MonitorId,
    ) -> Result<(), Self::Error>;

    fn set_pointer_position_relative(
        &mut self,
        x: u32,
        y: u32,
        window_id: WindowId,
    ) -> Result<(), Self::Error>;

    fn toggle_mouse(&mut self, button: MouseButton, down: bool) -> Result<(), Self::Error>;

    /// Returns Option<> representing if the type was found and Vec containing the content if it was found. Some(empty vec) is possible.
    fn clipboard_content(
        &mut self,
        type_name: &ClipboardType,
    ) -> Result<Option<Vec<u8>>, Self::Error>;

    fn set_clipboard_content(
        &mut self,
        type_name: &ClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error>;

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error>;

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error>;

    fn capture_monitor_frame(&mut self, monitor_id: MonitorId) -> Result<Frame, Self::Error>;

    fn update_monitor_frame(
        &mut self,
        monitor_id: u32,
        cap: &mut Frame,
    ) -> Result<(), Self::Error> {
        *cap = self.capture_monitor_frame(monitor_id)?;
        Ok(())
    }

    fn capture_window_frame(&mut self, window_id: WindowId) -> Result<Frame, Self::Error>;

    fn update_window_frame(
        &mut self,
        window_id: WindowId,
        cap: &mut Frame,
    ) -> Result<(), Self::Error> {
        *cap = self.capture_window_frame(window_id)?;
        Ok(())
    }
}

#[cfg(dummy_native)]
pub(crate) mod dummy {
    use super::*;
    use std::convert::Infallible;

    pub enum DummyApi {}

    impl DummyApi {
        pub fn new() -> Result<Self, Infallible> {
            unimplemented!()
        }
    }

    impl NativeApiTemplate for DummyApi {
        type Error = Infallible;

        fn key_toggle(&mut self, key: Key, down: bool) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn pointer_position(&mut self) -> Result<MousePosition, Self::Error> {
            unimplemented!()
        }

        fn set_pointer_position_absolute(
            &mut self,
            x: u32,
            y: u32,
            monitor_id: MonitorId,
        ) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn set_pointer_position_relative(
            &mut self,
            x: u32,
            y: u32,
            window_id: WindowId,
        ) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn toggle_mouse(&mut self, button: MouseButton, down: bool) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn clipboard_content(
            &mut self,
            type_name: &ClipboardType,
        ) -> Result<Option<Vec<u8>>, Self::Error> {
            unimplemented!()
        }

        fn set_clipboard_content(
            &mut self,
            type_name: &ClipboardType,
            content: &[u8],
        ) -> Result<(), Self::Error> {
            unimplemented!()
        }

        fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
            unimplemented!()
        }

        fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
            unimplemented!()
        }

        fn capture_monitor_frame(&mut self, monitor_id: MonitorId) -> Result<Frame, Self::Error> {
            unimplemented!()
        }

        fn capture_window_frame(&mut self, window_id: WindowId) -> Result<Frame, Self::Error> {
            unimplemented!()
        }
    }
}
