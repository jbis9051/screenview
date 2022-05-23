use crate::helper::rvd_helper::handshake;
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
    RvdMessage,
};
use peer::{
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
        ._handle(protocol_message, &mut write, &mut events)
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

    host._handle(msg, &mut events).expect("handler failed");
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
        ._handle(protocol_message, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 0);
    assert_eq!(write.len(), 1);

    let msg = write.remove(0);

    assert!(matches!(&msg, &RvdMessage::ProtocolVersionResponse(_)));

    host._handle(msg, &mut events).expect("handler failed");
    assert_eq!(events.len(), 0);
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
        ._handle(msg, &mut write, &mut events)
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
        ._handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdClientInform(RvdClientInform::MouseLocation(m)) if m == location)
    );


    let content: Vec<u8> = vec![1, 2, 3];

    let notification = ClipboardNotification {
        info: ClipboardMeta {
            clipboard_type: ClipboardType::Text,
            content_request: true,
        },
        type_exists: true,
        content: Some(content.clone()),
    };
    let msg = RvdMessage::ClipboardNotification(notification.clone());

    client
        ._handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdClientInform(RvdClientInform::ClipboardNotification(a, b)) if a == content && b == notification.info.clipboard_type)
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

    host._handle(
        RvdMessage::DisplayChangeReceived(DisplayChangeReceived {}),
        &mut events,
    )
    .expect("handler failed");
    assert_eq!(events.len(), 0);

    // MouseInput

    let mouse_input = MouseInput {
        display_id: 0,
        x_location: 1,
        y_location: 2,
        buttons_delta: ButtonsMask::empty(),
        buttons_state: ButtonsMask::empty(),
    };

    host._handle(RvdMessage::MouseInput(mouse_input), &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(matches!(
        event,
        InformEvent::RvdHostInform(RvdHostInform::MouseInput(event))
        if  event.button_state == ButtonsMask::empty()
            && event.button_delta == ButtonsMask::empty()
            && event.native_id == 10
            && event.display_type == DisplayType::Monitor
            && event.x_location == 1
            && event.y_location == 2
    ));

    // KeyInput

    let key_input = KeyInput {
        down: true,
        key: 20,
    };

    host._handle(RvdMessage::KeyInput(key_input.clone()), &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::KeyboardInput(k))  if k == key_input)
    );


    // ClipboardRequest

    host._handle(
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
    let content: Vec<u8> = vec![1, 2, 3];

    let notification = ClipboardNotification {
        info: ClipboardMeta {
            clipboard_type: ClipboardType::Text,
            content_request: true,
        },
        type_exists: true,
        content: Some(content.clone()),
    };
    let msg = RvdMessage::ClipboardNotification(notification.clone());

    host._handle(msg, &mut events).expect("handler failed");
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::ClipboardNotification(a, b)) if a == content && b == notification.info.clipboard_type)
    );
}


// TODO test permission errors
