use crate::api::*;

pub struct WindowsApi;

impl WindowsApi {
    pub fn new() -> Result<Self, Error> {
        unimplemented!()
    }
}

impl NativeApiTemplate for WindowsApi {
    type Error = Error;

    fn key_toggle(&self, key: Key, down: bool) {
        unimplemented!()
    }

    fn pointer_position(&self) -> Result<MousePosition, Error> {
        unimplemented!()
    }

    fn set_pointer_position(&self, pos: MousePosition) -> Result<(), Error> {
        unimplemented!()
    }

    fn toggle_mouse(&self, button: MouseButton, down: bool) -> Result<(), Error> {
        unimplemented!()
    }

    fn scroll_mouse(&self, scroll: MouseScroll) -> Result<(), Error> {
        unimplemented!()
    }

    fn clipboard_types(&self) -> Result<Vec<ClipboardType>, Error> {
        unimplemented!()
    }

    fn clipboard_content(&self, type_name: ClipboardType) -> Result<Vec<u8>, Error> {
        unimplemented!()
    }

    fn set_clipboard_content(
        &mut self,
        type_name: ClipboardType,
        content: &[u8],
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Error> {
        unimplemented!()
    }

    fn windows(&mut self) -> Result<Vec<Window>, Error> {
        unimplemented!()
    }

    fn capture_display_frame(&self, display: &Monitor) -> Result<Frame, Error> {
        unimplemented!()
    }

    fn update_display_frame(&self, display: &Monitor, cap: &mut Frame) -> Result<(), Error> {
        unimplemented!()
    }

    fn capture_window_frame(&self, display: &Window) -> Result<Frame, Error> {
        unimplemented!()
    }

    fn update_window_frame(&self, window: &Window, cap: &mut Frame) -> Result<(), Error> {
        unimplemented!()
    }
}

pub enum Error {}
