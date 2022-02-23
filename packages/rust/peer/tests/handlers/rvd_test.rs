use native::api::{
    ClipboardType,
    Frame,
    Key,
    Monitor,
    MouseButton,
    MousePosition,
    NativeApiTemplate,
    Window,
};
use peer::services::rvd::{RvdClientHandler, RvdHostHandler};
use std::convert::Infallible;

struct TesterNative {
    pub pointer_position: MousePosition,
    pub clipboard_content: Vec<u8>,
    pub monitors: Vec<Monitor>,
    pub windows: Vec<Window>,
    pub down_keys: Vec<Key>,
    pub mouse_button: Option<MouseButton>,
}

impl TesterNative {
    pub fn new() -> Self {
        TesterNative {
            pointer_position: MousePosition {
                x: 0,
                y: 0,
                monitor_id: 0,
            },
            clipboard_content: vec![],
            monitors: vec![],
            windows: vec![],
            down_keys: vec![],
            mouse_button: None,
        }
    }
}

impl NativeApiTemplate for TesterNative {
    type Error = Infallible;

    fn key_toggle(&mut self, key: Key, down: bool) -> Result<(), Self::Error> {
        if down {
            self.down_keys.push(key);
        } else {
            self.down_keys.retain(|k| *k != key);
        }
        Ok(())
    }

    fn pointer_position(&mut self) -> Result<MousePosition, Self::Error> {
        Ok(self.pointer_position)
    }

    fn set_pointer_position(&mut self, pos: MousePosition) -> Result<(), Self::Error> {
        self.pointer_position = pos;
        Ok(())
    }

    fn toggle_mouse(&mut self, button: MouseButton, down: bool) -> Result<(), Self::Error> {
        self.mouse_button = if down { Some(button) } else { None };
        Ok(())
    }

    fn clipboard_content(&mut self, type_name: &ClipboardType) -> Result<Vec<u8>, Self::Error> {
        Ok(self.clipboard_content.clone())
    }

    fn set_clipboard_content(
        &mut self,
        type_name: &ClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error> {
        self.clipboard_content = content.to_vec();
        Ok(())
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
        Ok(self.monitors.clone())
    }

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
        Ok(self.windows.clone())
    }

    fn capture_display_frame(&mut self, display: &Monitor) -> Result<Frame, Self::Error> {
        unimplemented!()
    }

    fn capture_window_frame(&mut self, display: &Window) -> Result<Frame, Self::Error> {
        unimplemented!()
    }
}


#[test]
fn test_rvd_handshake() {
    let host_native = TesterNative::new();
    let host = RvdHostHandler::new(host_native);

    let client_native = TesterNative::new();
    let client = RvdClientHandler::new(client_native);
}
