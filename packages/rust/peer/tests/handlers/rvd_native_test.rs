use crate::helper::rvd_helper::handshake;
use common::messages::rvd::{
    ButtonsMask,
    ClipboardMeta,
    ClipboardNotification,
    ClipboardRequest,
    ClipboardType,
    DisplayChangeReceived,
    KeyInput,
    MouseInput,
    RvdMessage,
};
use native::api::{
    BGRAFrame,
    ClipboardType as NativeClipboardType,
    Key,
    Monitor,
    MonitorId,
    MouseButton,
    MousePosition,
    NativeApiTemplate,
    Window,
    WindowId,
};
use peer::{
    helpers::rvd_native_helper::{rvd_client_native_helper, rvd_host_native_helper},
    rvd::{DisplayType, RvdClientHandler, RvdDisplay, RvdHostHandler},
};
use std::convert::Infallible;


#[test]
fn test_client() {
    let mut write = Vec::new();
    let mut events = Vec::new();
    let mut native = TesterNative::new();
    let mut client = RvdClientHandler::new();
    handshake(None, Some(&mut client));

    let clipboard: Vec<u8> = vec![1, 2, 3];

    let msg = RvdMessage::ClipboardNotification(ClipboardNotification {
        info: ClipboardMeta {
            clipboard_type: ClipboardType::Text,
            content_request: true,
        },
        type_exists: true,
        content: Some(clipboard.clone()),
    });

    assert_ne!(native.clipboard_content, clipboard);

    rvd_client_native_helper(msg, &mut write, &mut events, &mut client, &mut native)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);

    assert_eq!(native.clipboard_content, clipboard);
}

#[test]
fn test_host_notification() {
    let mut write = Vec::new();
    let mut events = Vec::new();
    let mut native = TesterNative::new();
    let mut host = RvdHostHandler::new();
    handshake(Some(&mut host), None);
    host.set_controllable(true);
    host.set_clipboard_readable(true);

    let clipboard: Vec<u8> = vec![1, 2, 3];

    let msg = RvdMessage::ClipboardNotification(ClipboardNotification {
        info: ClipboardMeta {
            clipboard_type: ClipboardType::Text,
            content_request: true,
        },
        type_exists: true,
        content: Some(clipboard.clone()),
    });

    assert_ne!(native.clipboard_content, clipboard);

    rvd_host_native_helper(msg, &mut write, &mut events, &mut host, &mut native)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);

    assert_eq!(native.clipboard_content, clipboard);
}

#[test]
fn test_host_key_input() {
    let mut write = Vec::new();
    let mut events = Vec::new();
    let mut native = TesterNative::new();
    let mut host = RvdHostHandler::new();
    handshake(Some(&mut host), None);
    host.set_controllable(true);

    let msg = RvdMessage::KeyInput(KeyInput {
        down: true,
        key: 40,
    });

    assert!(!native.down_keys.contains(&40));

    rvd_host_native_helper(msg, &mut write, &mut events, &mut host, &mut native)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);

    assert!(native.down_keys.contains(&40));

    let msg = RvdMessage::KeyInput(KeyInput {
        down: false,
        key: 40,
    });
    rvd_host_native_helper(msg, &mut write, &mut events, &mut host, &mut native)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);

    assert!(!native.down_keys.contains(&40));
}


#[test]
fn test_host_mouse_input() {
    let mut write = Vec::new();
    let mut events = Vec::new();
    let mut native = TesterNative::new();
    let mut host = RvdHostHandler::new();
    handshake(Some(&mut host), None);
    host.set_controllable(true);
    let monitor = &native.monitors[0];
    host.share_display(RvdDisplay {
        native_id: monitor.id,
        name: monitor.name.clone(),
        display_type: DisplayType::Monitor,
        width: monitor.width as u16,
        height: monitor.height as u16,
    });
    let update = host.display_update();
    host._handle(
        RvdMessage::DisplayChangeReceived(DisplayChangeReceived {}),
        &mut events,
    )
    .expect("handler failed");

    let update = match update {
        RvdMessage::DisplayChange(change) => change,
        _ => panic!("bad update received"),
    };


    let msg = RvdMessage::MouseInput(MouseInput {
        display_id: update.display_information[0].display_id,
        x_location: 150,
        y_location: 300,
        buttons_delta: ButtonsMask::empty(),
        buttons_state: ButtonsMask::empty(),
    });

    assert_eq!(native.pointer_x, 0);
    assert_eq!(native.pointer_y, 0);

    rvd_host_native_helper(msg, &mut write, &mut events, &mut host, &mut native)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);

    assert_eq!(native.pointer_x, 150);
    assert_eq!(native.pointer_y, 300);

    // TODO test button mask
}


#[test]
fn test_host_clipboard_request() {
    let mut write = Vec::new();
    let mut events = Vec::new();
    let mut native = TesterNative::new();
    let mut host = RvdHostHandler::new();
    handshake(Some(&mut host), None);
    host.set_clipboard_readable(true);

    let clip: Vec<u8> = vec![10, 20, 30];

    native.clipboard_content = clip.clone();

    let msg = RvdMessage::ClipboardRequest(ClipboardRequest {
        info: ClipboardMeta {
            clipboard_type: ClipboardType::Text,
            content_request: true,
        },
    });

    rvd_host_native_helper(msg, &mut write, &mut events, &mut host, &mut native)
        .expect("handler failed");

    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 0);

    let msg = write.remove(0);

    assert!(matches!(msg,
        RvdMessage::ClipboardNotification(notificaiton)
        if notificaiton.type_exists
        && notificaiton.content == Some(clip)
        && notificaiton.info.content_request
        && notificaiton.info.clipboard_type == ClipboardType::Text
    ))
}

#[derive(Debug)]
struct TesterNative {
    pub pointer_x: u32,
    pub pointer_y: u32,
    pub clipboard_content: Vec<u8>,
    pub monitors: Vec<Monitor>,
    pub windows: Vec<Window>,
    pub down_keys: Vec<Key>,
    pub mouse_button: Option<MouseButton>,
}

impl TesterNative {
    pub fn new() -> Self {
        TesterNative {
            pointer_x: 0,
            pointer_y: 0,
            clipboard_content: vec![],
            monitors: vec![
                Monitor {
                    id: 1,
                    name: "Mock Display 1".to_string(),
                    width: 1000,
                    height: 1000,
                },
                Monitor {
                    id: 2,
                    name: "Mock Display 2".to_string(),
                    width: 1980,
                    height: 1080,
                },
            ],
            windows: vec![
                Window {
                    id: 1,
                    name: "Mock Window 1".to_string(),
                    width: 100,
                    height: 100,
                },
                Window {
                    id: 2,
                    name: "Mock Window 1".to_string(),
                    width: 100,
                    height: 100,
                },
            ],
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

    fn pointer_position(&mut self, _windows: &[WindowId]) -> Result<MousePosition, Self::Error> {
        Ok(MousePosition {
            x: 0,
            y: 0,
            monitor_id: 0,
            window_relatives: vec![],
        })
    }

    fn set_pointer_position_absolute(
        &mut self,
        x: u32,
        y: u32,
        _monitor_id: MonitorId,
    ) -> Result<(), Self::Error> {
        self.pointer_x = x;
        self.pointer_y = y;
        Ok(())
    }

    fn set_pointer_position_relative(
        &mut self,
        x: u32,
        y: u32,
        _window_id: WindowId,
    ) -> Result<(), Self::Error> {
        self.set_pointer_position_absolute(x, y, 0)
    }

    fn toggle_mouse(
        &mut self,
        button: MouseButton,
        down: bool,
        _window_id: Option<WindowId>,
    ) -> Result<(), Self::Error> {
        self.mouse_button = if down { Some(button) } else { None };
        Ok(())
    }

    fn clipboard_content(
        &mut self,
        _type_name: &NativeClipboardType,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(Some(self.clipboard_content.clone()))
    }

    fn set_clipboard_content(
        &mut self,
        _type_name: &NativeClipboardType,
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

    fn capture_monitor_frame(&mut self, _monitor_id: MonitorId) -> Result<BGRAFrame, Self::Error> {
        unimplemented!()
    }

    fn capture_window_frame(&mut self, _window_id: WindowId) -> Result<BGRAFrame, Self::Error> {
        unimplemented!()
    }
}
