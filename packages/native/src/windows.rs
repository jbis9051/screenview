use crate::api::*;

pub struct WindowsApi;

impl NativeApiTemplate for WindowsApi {
    type Error = std::convert::Infallible;

    fn new() -> Result<Self, Self::Error> {
        unimplemented!()
    }

    fn key_toggle(&self, key: Key, down: bool) {
        unimplemented!()
    }

    fn pointer_position(&self) -> Result<MousePosition, Self::Error> {
        unimplemented!()
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
        unimplemented!()
    }

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
        unimplemented!()
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
