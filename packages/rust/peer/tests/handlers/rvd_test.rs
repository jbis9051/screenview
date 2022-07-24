use crate::helper::rvd_helper::handshake;
use common::messages::rvd::{
    AccessMask,
    ButtonsMask,
    ClipboardMeta,
    ClipboardNotification,
    ClipboardRequest,
    ClipboardType,
    DisplayShare,
    DisplayShareAck,
    KeyInput,
    MouseInput,
    MouseLocation,
    PermissionMask,
    ProtocolVersion,
    RvdMessage,
};
use peer::{
    rvd::{RvdClientHandler, RvdClientInform, RvdHandlerTrait, RvdHostHandler, RvdHostInform},
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

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 0);
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

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 0);
    assert_eq!(write.len(), 1);

    let msg = write.remove(0);

    assert!(matches!(&msg, &RvdMessage::UnreliableAuthInitial(_)));

    client
        .handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(events.len(), 0);
    assert_eq!(write.len(), 1);


    let msg = write.remove(0);

    assert!(matches!(&msg, &RvdMessage::UnreliableAuthInter(_)));

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");

    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);
    assert!(matches!(
        event,
        InformEvent::RvdHostInform(RvdHostInform::HandshakeComplete)
    ));

    let msg = write.remove(0);
    assert!(matches!(&msg, &RvdMessage::UnreliableAuthFinal(_)));

    client
        .handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);
    let event = events.remove(0);
    assert!(matches!(
        event,
        InformEvent::RvdClientInform(RvdClientInform::HandshakeComplete)
    ));
}

#[test]
fn test_rvd_client() {
    let mut write = Vec::new();
    let mut events = Vec::new();

    let mut client = RvdClientHandler::new();
    handshake(None, Some(&mut client));

    let change = DisplayShare {
        display_id: 0,
        access: AccessMask::empty(),
        name: "testing1".to_string(),
    };

    let msg = RvdMessage::DisplayShare(change.clone());

    client
        .handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 1);
    assert_eq!(events.len(), 1);

    let msg = write.remove(0);
    let event = events.remove(0);

    assert!(matches!(msg, RvdMessage::DisplayShareAck(_)));
    assert!(
        matches!(event, InformEvent::RvdClientInform(RvdClientInform::DisplayShare(c)) if c == change)
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
        .handle(msg, &mut write, &mut events)
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
    let mut write = Vec::new();
    let mut events = Vec::new();

    let mut host = RvdHostHandler::new();
    handshake(Some(&mut host), None);

    // share_display
    // TODO some more extensive testing of flushing and shtuff

    let (display_id, msg) = host
        .share_display("fake_display".to_string(), AccessMask::CONTROLLABLE)
        .expect("share_display failed");

    assert!(matches!(msg, RvdMessage::DisplayShare(_)));

    host.handle(
        RvdMessage::DisplayShareAck(DisplayShareAck { display_id }),
        &mut write,
        &mut events,
    )
    .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 0);

    // MouseInput

    let mouse_input = MouseInput {
        display_id: 0,
        x_location: 1,
        y_location: 2,
        buttons_delta: ButtonsMask::empty(),
        buttons_state: ButtonsMask::empty(),
    };

    host.handle(RvdMessage::MouseInput(mouse_input), &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(matches!(
        event,
        InformEvent::RvdHostInform(RvdHostInform::MouseInput(event))
        if  event.button_state == ButtonsMask::empty()
            && event.button_delta == ButtonsMask::empty()
            && event.display_id == display_id
            && event.x_location == 1
            && event.y_location == 2
    ));

    // KeyInput

    let key_input = KeyInput {
        down: true,
        key: 20,
    };

    host.handle(
        RvdMessage::KeyInput(key_input.clone()),
        &mut write,
        &mut events,
    )
    .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::KeyboardInput(k))  if k == key_input)
    );


    // ClipboardRequest

    host.set_permissions(PermissionMask::CLIPBOARD_READ);

    host.handle(
        RvdMessage::ClipboardRequest(ClipboardRequest {
            info: ClipboardMeta {
                clipboard_type: ClipboardType::Text,
                content_request: false,
            },
        }),
        &mut write,
        &mut events,
    )
    .expect("handler failed");

    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::ClipboardRequest(b, t))  if !b && t == ClipboardType::Text)
    );

    // ClipboardNotification
    host.set_permissions(PermissionMask::CLIPBOARD_WRITE);

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

    host.handle(msg, &mut write, &mut events)
        .expect("handler failed");
    assert_eq!(write.len(), 0);
    assert_eq!(events.len(), 1);

    let event = events.remove(0);

    assert!(
        matches!(event, InformEvent::RvdHostInform(RvdHostInform::ClipboardNotification(a, b)) if a == content && b == notification.info.clipboard_type)
    );
}


// TODO test permission errors
