use common::messages::rvd::{
    AccessMask,
    ButtonsMask,
    ClipboardMeta,
    ClipboardNotification,
    ClipboardRequest,
    ClipboardType,
    DisplayChange,
    DisplayChangeReceived,
    DisplayInformation,
    KeyInput,
    MouseInput,
    MouseLocation,
    ProtocolVersion,
    ProtocolVersionResponse,
    RvdMessage,
};
use native::api::MousePosition;
use peer::services::{
    rvd::{
        DisplayType,
        RvdClientHandler,
        RvdClientInform,
        RvdDisplay,
        RvdHostHandler,
        RvdHostInform,
    },
    InformEvent,
};

#[test]
fn test_rvd_version_mismatch() {
    let mut write = Vec::new();
    let mut events = Vec::new();

    let mut host = RvdHostHandler::new();

    let mut client = RvdClientHandler::new();


    let protocol_message = RvdMessage::ProtocolVersion(ProtocolVersion {
        version: "badversion".to_string(),
    });

    client
        .handle(protocol_message, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 1);
    assert_eq!(write.len(), 1);
    let event = events.remove(0);
    assert!(matches!(
        event,
        InformEvent::RvdClientInform(RvdClientInform::VersionBad)
    ));
    let msg = write.remove(0);
    assert!(matches!(&msg, &RvdMessage::ProtocolVersionResponse(_)));

    host.handle(msg, &mut events).expect("handler failed");
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::RvdHostInform(RvdHostInform::VersionBad)
    ));
}

#[test]
fn test_rvd_handshake() {
    let mut write = Vec::new();
    let mut events = Vec::new();

    let mut host = RvdHostHandler::new();

    let mut client = RvdClientHandler::new();


    let protocol_message = RvdHostHandler::protocol_version();

    client
        .handle(protocol_message, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 0);
    assert_eq!(write.len(), 1);

    let msg = write.remove(0);

    assert!(matches!(&msg, &RvdMessage::ProtocolVersionResponse(_)));

    host.handle(msg, &mut events).expect("handler failed");
    assert_eq!(events.len(), 0);
}

fn handshake(host: Option<&mut RvdHostHandler>, client: Option<&mut RvdClientHandler>) {
    let mut write = Vec::new();
    let mut events = Vec::new();

    if let Some(client) = client {
        let protocol_message = RvdHostHandler::protocol_version();

        client
            .handle(protocol_message, &mut write, &mut events)
            .expect("handler failed");
    }

    if let Some(host) = host {
        let msg = RvdMessage::ProtocolVersionResponse(ProtocolVersionResponse { ok: true });
        host.handle(msg, &mut events).expect("handler failed");
    }
}

#[test]
fn test_rvd_client() {
    let mut write = Vec::new();
    let mut events = Vec::new();

    let mut client = RvdClientHandler::new();
    handshake(None, Some(&mut client));

    let change = DisplayChange {
        clipboard_readable: false,
        display_information: vec![DisplayInformation {
            display_id: 0,
            width: 10,
            height: 20,
            cell_width: 30,
            cell_height: 40,
            access: AccessMask::FLUSH,
            name: "testing1".to_string(),
        }],
    };

    let msg = RvdMessage::DisplayChange(change.clone());

    client
        .handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 1);

    let msg = write.remove(0);
    let event = events.remove(0);

    assert!(matches!(msg, RvdMessage::DisplayChangeReceived(_)));
    assert!(
        matches!(event, InformEvent::RvdClientInform(RvdClientInform::DisplayChange(c)) if c == change)
    );


    let location = MouseLocation {
        display_id: 1,
        x_location: 2,
        y_location: 3,
    };
    let msg = RvdMessage::MouseLocation(location.clone());

    client
        .handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdClientInform(RvdClientInform::MouseLocation(m)) if m == location)
    );


    let notification = ClipboardNotification {
        info: ClipboardMeta {
            clipboard_type: ClipboardType::Text,
            content_request: false,
        },
        content: None,
    };
    let msg = RvdMessage::ClipboardNotification(notification.clone());

    client
        .handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdClientInform(RvdClientInform::ClipboardNotification(a, b)) if a == notification.content && b == notification.info.clipboard_type)
    );
}


#[test]
fn test_rvd_host() {
    let mut events = Vec::new();

    let mut host = RvdHostHandler::new();
    host.set_clipboard_readable(true);
    host.set_controllable(true);
    handshake(Some(&mut host), None);

    // share_display
    // TODO some more extensive testing of flushing and shtuff

    host.share_display(RvdDisplay {
        native_id: 10,
        name: "fakedisplay".to_string(),
        display_type: DisplayType::Monitor,
        width: 0,
        height: 0,
    });
    let msg = host.display_update();
    assert!(matches!(msg, RvdMessage::DisplayChange(_)));

    host.handle(
        RvdMessage::DisplayChangeReceived(DisplayChangeReceived {}),
        &mut events,
    )
    .expect("handler failed");
    assert_eq!(events.len(), 0);

    // MouseInput

    let mouse_position = MousePosition {
        x: 1,
        y: 2,
        monitor_id: 3,
    };

    let buttons = ButtonsMask::empty();

    let mouse_input = MouseInput {
        display_id: mouse_position.monitor_id,
        x_location: mouse_position.x as u16, // TODO problem conversion
        y_location: mouse_position.y as u16, // TODO problem conversion
        buttons,
    };

    host.handle(RvdMessage::MouseInput(mouse_input), &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::MouseInput(m, mask))  if m == mouse_position && mask == buttons)
    );

    // KeyInput

    let key_input = KeyInput {
        down: true,
        key: 20,
    };

    host.handle(RvdMessage::KeyInput(key_input.clone()), &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::KeyboardInput(k))  if k == key_input)
    );


    // ClipboardRequest

    host.handle(
        RvdMessage::ClipboardRequest(ClipboardRequest {
            info: ClipboardMeta {
                clipboard_type: ClipboardType::Text,
                content_request: false,
            },
        }),
        &mut events,
    )
    .expect("handler failed");

    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::ClipboardRequest(b, t))  if !b && t == ClipboardType::Text)
    );

    // ClipboardNotification

    let notification = ClipboardNotification {
        info: ClipboardMeta {
            clipboard_type: ClipboardType::Text,
            content_request: false,
        },
        content: None,
    };
    let msg = RvdMessage::ClipboardNotification(notification.clone());

    host.handle(msg, &mut events).expect("handler failed");
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::ClipboardNotification(a, b)) if a == notification.content && b == notification.info.clipboard_type)
    );
}


// TODO test permission errors
