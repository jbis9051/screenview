extern crate core;

use common::{
    chrono::{MAX_DATETIME, MIN_DATETIME},
    constants::SVSC_VERSION,
    messages::{
        svsc::{
            EstablishSessionNotification,
            EstablishSessionResponse,
            EstablishSessionStatus,
            KeepAlive,
            LeaseExtensionResponse,
            LeaseId,
            LeaseResponse,
            LeaseResponseData,
            ProtocolVersion,
            SessionData,
            SessionDataReceive,
            SessionEnd,
            SvscMessage,
        },
        Data,
    },
};
use peer::{
    svsc_handler::{SvscHandler, SvscInform},
    InformEvent,
};
use std::borrow::Cow;

#[test]
fn test_protocol_mismatch() {
    let mut handler = SvscHandler::new();
    let mut events = Vec::new();
    let mut write = Vec::new();

    let message = SvscMessage::ProtocolVersion(ProtocolVersion {
        version: "badversion".to_string(),
    });
    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::SvscInform(SvscInform::VersionBad)
    ));


    assert_eq!(write.len(), 1);
    let msg = match &write[0] {
        SvscMessage::ProtocolVersionResponse(msg) => msg,
        _ => panic!("wrong message returned: {:?}", write[0]),
    };

    assert!(!msg.ok);
}

fn handshake(handler: &mut SvscHandler) {
    let message = SvscMessage::ProtocolVersion(ProtocolVersion {
        version: SVSC_VERSION.to_string(),
    });
    let mut events = Vec::new();
    let mut write = Vec::new();
    handler.handle(message, &mut write, &mut events).unwrap();
}

#[test]
fn test_protocol_version() {
    let mut handler = SvscHandler::new();
    let mut events = Vec::new();
    let mut write = Vec::new();

    let message = SvscMessage::ProtocolVersion(ProtocolVersion {
        version: SVSC_VERSION.to_string(),
    });
    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    assert_eq!(events.len(), 0);

    assert_eq!(write.len(), 1);
    let msg = match &write[0] {
        SvscMessage::ProtocolVersionResponse(msg) => msg,
        _ => panic!("wrong message returned: {:?}", write[0]),
    };

    assert!(msg.ok);
}

#[test]
fn test_lease() {
    let mut handler = SvscHandler::new();
    let mut events = Vec::new();
    let mut write = Vec::new();

    handshake(&mut handler);

    // lease request
    let message = handler.lease_request(None);

    assert!(matches!(message, SvscMessage::LeaseRequest(_)));

    // lease response
    let mut response_data = LeaseResponseData {
        id: 10,
        cookie: [12u8; 24],
        expiration: MIN_DATETIME,
    };

    let message = SvscMessage::LeaseResponse(LeaseResponse {
        response_data: Some(response_data.clone()),
    });

    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::SvscInform(SvscInform::LeaseUpdate)
    ));

    assert_eq!(write.len(), 0);
    let lease = handler.lease().expect("expected peer_id");
    assert_eq!(lease, &response_data);

    events.clear();

    // lease extension request
    let message = handler.lease_extension_request(response_data.cookie);

    assert!(matches!(message, SvscMessage::LeaseExtensionRequest(_)));

    // lease extension response
    let message = SvscMessage::LeaseExtensionResponse(LeaseExtensionResponse {
        new_expiration: Some(MAX_DATETIME),
    });

    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::SvscInform(SvscInform::LeaseUpdate)
    ));

    assert_eq!(write.len(), 0);
    let lease = handler.lease().expect("expected lease");
    response_data.expiration = MAX_DATETIME;
    assert_eq!(lease, &response_data);
}

#[test]
fn test_establish_session() {
    let mut handler = SvscHandler::new();
    let mut events = Vec::new();
    let mut write = Vec::new();

    handshake(&mut handler);

    let lease_id: LeaseId = [1, 2, 3, 4];

    // establish session request
    let message = handler.establish_session_request(lease_id);

    assert!(matches!(message, SvscMessage::EstablishSessionRequest(_)));

    // establish session response
    let session_data = SessionData {
        session_id: [1u8; 16],
        peer_id: [2u8; 16],
        peer_key: [3u8; 16],
    };
    let message = SvscMessage::EstablishSessionResponse(EstablishSessionResponse {
        lease_id,
        status: EstablishSessionStatus::Success,
        response_data: Some(Box::new(session_data.clone())),
    });

    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::SvscInform(SvscInform::SessionUpdate)
    ));

    assert_eq!(write.len(), 0);
    let session = handler.session().expect("expected session");
    assert_eq!(session, &session_data);
}

#[test]
fn test_establish_session_notification() {
    let mut handler = SvscHandler::new();
    let mut events = Vec::new();
    let mut write = Vec::new();

    handshake(&mut handler);

    let session_data = SessionData {
        session_id: [1u8; 16],
        peer_id: [2u8; 16],
        peer_key: [3u8; 16],
    };


    let message = SvscMessage::EstablishSessionNotification(EstablishSessionNotification {
        session_data: session_data.clone(),
    });

    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::SvscInform(SvscInform::SessionUpdate)
    ));

    assert_eq!(write.len(), 0);
    let session = handler.session().expect("expected session");
    assert_eq!(session, &session_data);
}

#[test]
fn test_session_end() {
    let mut handler = SvscHandler::new();
    let mut events = Vec::new();
    let mut write = Vec::new();

    handshake(&mut handler);

    let session_data = SessionData {
        session_id: [1u8; 16],
        peer_id: [2u8; 16],
        peer_key: [3u8; 16],
    };

    // setup
    let message =
        SvscMessage::EstablishSessionNotification(EstablishSessionNotification { session_data });

    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    events.clear();

    // test
    assert!(handler.session().is_some());
    let message = SvscMessage::SessionEnd(SessionEnd {});

    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        InformEvent::SvscInform(SvscInform::SessionUpdate)
    ));
    assert!(handler.session().is_none());
}

#[test]
fn test_session_data() {
    let mut handler = SvscHandler::new();
    let mut events = Vec::new();
    let mut write = Vec::new();

    handshake(&mut handler);

    let session_data = SessionData {
        session_id: [1u8; 16],
        peer_id: [2u8; 16],
        peer_key: [3u8; 16],
    };

    // setup
    let message =
        SvscMessage::EstablishSessionNotification(EstablishSessionNotification { session_data });

    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    events.clear();

    // test
    let data = vec![1, 2, 3, 4, 5, 6];
    let message = SvscMessage::SessionDataReceive(SessionDataReceive {
        data: Data(Cow::Owned(data.clone())),
    });

    let inner = handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .expect("expected data");
    assert_eq!(events.len(), 0);
    assert_eq!(write.len(), 0);
    assert_eq!(data, &*inner);
}


#[test]
fn test_keepalive() {
    let mut handler = SvscHandler::new();
    let mut events = Vec::new();
    let mut write = Vec::new();

    let message = SvscMessage::KeepAlive(KeepAlive {});

    assert!(handler
        .handle(message, &mut write, &mut events)
        .expect("handler failed")
        .is_none());

    assert_eq!(events.len(), 0);
    assert_eq!(write.len(), 1);
    assert!(matches!(write[0], SvscMessage::KeepAlive(_)));
}
