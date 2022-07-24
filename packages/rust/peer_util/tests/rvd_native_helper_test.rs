use common::messages::rvd::{
    AccessMask,
    ButtonsMask,
    ClipboardMeta,
    ClipboardNotification,
    ClipboardRequest,
    ClipboardType,
    DisplayShareAck,
    KeyInput,
    MouseInput,
    PermissionMask,
    ProtocolVersionResponse,
    RvdMessage,
    UnreliableAuthFinal,
    UnreliableAuthInitial,
    UnreliableAuthInter,
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
    NativeId,
    Window,
    WindowId,
};
use peer::{
    rvd::{RvdClientHandler, RvdHandlerTrait, RvdHostHandler},
    InformEvent,
};
use peer_util::rvd_native_helper::{rvd_client_native_helper, rvd_host_native_helper};
use std::{collections::HashMap, convert::Infallible};

// TODO consider not involving the RvdHandlers and just testing rvd_{client, host}_native_helper
pub fn handshake(host: Option<&mut RvdHostHandler>, client: Option<&mut RvdClientHandler>) {
    let mut write = Vec::new();
    let mut events = Vec::new();

    if let Some(client) = client {
        let protocol_message = RvdHostHandler::protocol_version();

        client
            .handle(protocol_message, &mut write, &mut events)
            .expect("handler failed");
        write.clear();
        client
            .handle(
                RvdMessage::UnreliableAuthInitial(UnreliableAuthInitial {
                    challenge: *b"challengechallen",
                    zero: [0u8; 16],
                }),
                &mut write,
                &mut events,
            )
            .expect("handler failed");
        let msg = write.remove(0);
        let challenge = match msg {
            RvdMessage::UnreliableAuthInter(UnreliableAuthInter { challenge, .. }) => challenge,
            _ => panic!("wrong message type"),
        };
        client
            .handle(
                RvdMessage::UnreliableAuthFinal(UnreliableAuthFinal {
                    response: challenge,
                }),
                &mut write,
                &mut events,
            )
            .expect("handler failed");
    }

    if let Some(host) = host {
        let msg = RvdMessage::ProtocolVersionResponse(ProtocolVersionResponse { ok: true });
        host.handle(msg, &mut write, &mut events)
            .expect("handler failed");
        let msg = write.remove(0);
        let challange = match msg {
            RvdMessage::UnreliableAuthInitial(UnreliableAuthInitial { challenge, .. }) => challenge,
            _ => panic!("wrong message type"),
        };
        host.handle(
            RvdMessage::UnreliableAuthInter(UnreliableAuthInter {
                challenge: [0u8; 16],
                response: challange,
            }),
            &mut write,
            &mut events,
        )
        .expect("handler failed");
    }
}

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

    client
        .handle(msg, &mut write, &mut events)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    let event = match event {
        InformEvent::RvdClientInform(e) => e,
        _ => panic!("unexpected event"),
    };

    assert!(rvd_client_native_helper(event, &mut native)
        .expect("handler failed")
        .is_none());

    assert_eq!(native.clipboard_content, clipboard);
}

#[test]
fn test_host_notification() {
    let mut write = Vec::new();
    let mut events = Vec::new();
    let mut native = TesterNative::new();
    let mut host = RvdHostHandler::new();
    handshake(Some(&mut host), None);

    host.set_permissions(PermissionMask::CLIPBOARD_WRITE);

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

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);
    let event = match event {
        InformEvent::RvdHostInform(e) => e,
        _ => panic!("unexpected event"),
    };


    let (event, msg) =
        rvd_host_native_helper(event, &mut native, &HashMap::new()).expect("handler failed");

    assert!(event.is_none());
    assert!(msg.is_none());

    assert_eq!(native.clipboard_content, clipboard);
}

#[test]
fn test_host_key_input() {
    let mut write = Vec::new();
    let mut events = Vec::new();
    let mut native = TesterNative::new();
    let mut host = RvdHostHandler::new();

    handshake(Some(&mut host), None);

    host.share_display("test".to_string(), AccessMask::CONTROLLABLE)
        .expect("share_display failed");

    let msg = RvdMessage::KeyInput(KeyInput {
        down: true,
        key: 40,
    });

    assert!(!native.down_keys.contains(&40));

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);
    let event = match event {
        InformEvent::RvdHostInform(e) => e,
        _ => panic!("unexpected event"),
    };


    let (event, msg) =
        rvd_host_native_helper(event, &mut native, &HashMap::new()).expect("handler failed");

    assert!(event.is_none());
    assert!(msg.is_none());

    assert!(native.down_keys.contains(&40));

    let msg = RvdMessage::KeyInput(KeyInput {
        down: false,
        key: 40,
    });

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);
    let event = match event {
        InformEvent::RvdHostInform(e) => e,
        _ => panic!("unexpected event"),
    };


    let (event, msg) =
        rvd_host_native_helper(event, &mut native, &HashMap::new()).expect("handler failed");

    assert!(event.is_none());
    assert!(msg.is_none());

    assert!(!native.down_keys.contains(&40));
}


#[test]
fn test_host_mouse_input() {
    let mut write = Vec::new();
    let mut events = Vec::new();
    let mut native = TesterNative::new();
    let mut host = RvdHostHandler::new();
    handshake(Some(&mut host), None);

    let mut map = HashMap::new();
    let monitor = &native.monitors[0];

    let (display_id, _) = host
        .share_display(monitor.name.clone(), AccessMask::CONTROLLABLE)
        .expect("share_display failed");

    map.insert(display_id, NativeId::Monitor(monitor.id));

    host.handle(
        RvdMessage::DisplayShareAck(DisplayShareAck { display_id }),
        &mut write,
        &mut events,
    )
    .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);

    let msg = RvdMessage::MouseInput(MouseInput {
        display_id,
        x_location: 150,
        y_location: 300,
        buttons_delta: ButtonsMask::empty(),
        buttons_state: ButtonsMask::empty(),
    });

    assert_eq!(native.pointer_x, 0);
    assert_eq!(native.pointer_y, 0);

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);
    let event = match event {
        InformEvent::RvdHostInform(e) => e,
        _ => panic!("unexpected event"),
    };


    let (event, msg) = rvd_host_native_helper(event, &mut native, &map).expect("handler failed");

    assert!(event.is_none());
    assert!(msg.is_none());

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

    host.set_permissions(PermissionMask::CLIPBOARD_READ);

    let clip: Vec<u8> = vec![10, 20, 30];

    native.clipboard_content = clip.clone();

    let msg = RvdMessage::ClipboardRequest(ClipboardRequest {
        info: ClipboardMeta {
            clipboard_type: ClipboardType::Text,
            content_request: true,
        },
    });

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);
    let event = match event {
        InformEvent::RvdHostInform(e) => e,
        _ => panic!("unexpected event"),
    };


    let (event, msg) =
        rvd_host_native_helper(event, &mut native, &HashMap::new()).expect("handler failed");

    assert!(event.is_none());

    let msg = match msg {
        Some(m) => m,
        _ => panic!("expected a message but found none"),
    };

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
