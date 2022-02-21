mod helper;
use crate::helper::test_write;
use byteorder::{LittleEndian, ReadBytesExt};
use common::messages::{svsc::*, MessageComponent};
use std::io::Cursor;

#[test]
fn test_version() {
    let bytes = include_bytes!("binary/svsc/protocol_version.bin");
    let message: ProtocolVersion = ProtocolVersion::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.version, String::from_utf8(bytes.to_vec()).unwrap());
    test_write(&message, bytes);
}

#[test]
fn test_version_response() {
    let bytes = include_bytes!("binary/svsc/protocol_version_response.bin");
    let message: ProtocolVersionResponse =
        ProtocolVersionResponse::read(&mut Cursor::new(bytes)).unwrap();
    assert!(message.ok);
    test_write(&message, bytes);
}

#[test]
fn test_lease_request_cookie() {
    let bytes = include_bytes!("binary/svsc/lease_request_cookie.bin");
    let message: LeaseRequest = LeaseRequest::read(&mut Cursor::new(bytes)).unwrap();
    assert!(message.cookie.is_some());
    assert_eq!(&message.cookie.unwrap(), b"cookiecookiecookiecookie"); // Yum!
    test_write(&message, bytes);
}

#[test]
fn test_lease_request_no_cookie() {
    let bytes = include_bytes!("binary/svsc/lease_request_no_cookie.bin");
    let message: LeaseRequest = LeaseRequest::read(&mut Cursor::new(bytes)).unwrap();
    assert!(message.cookie.is_none());
    test_write(&message, bytes);
}

#[test]
fn test_lease_response_accepted() {
    let bytes = include_bytes!("binary/svsc/lease_response_accepted.bin");
    let message: LeaseResponse = LeaseResponse::read(&mut Cursor::new(bytes)).unwrap();
    assert!(message.response_data.is_some());
    assert_eq!(
        &message.response_data.as_ref().unwrap().id,
        &b"idid".as_slice().read_u32::<LittleEndian>().unwrap(),
        "id"
    );
    assert_eq!(
        &message.response_data.as_ref().unwrap().cookie,
        b"cookiecookiecookiecookie",
        "cookie"
    );
    test_write(&message, bytes);
}

#[test]
fn test_lease_response_rejected() {
    let bytes = include_bytes!("binary/svsc/lease_response_rejected.bin");
    let message: LeaseResponse = LeaseResponse::read(&mut Cursor::new(bytes)).unwrap();
    assert!(message.response_data.is_none());
    test_write(&message, bytes);
}

#[test]
fn test_lease_extend_request() {
    let bytes = include_bytes!("binary/svsc/lease_extend_request.bin");
    let message: LeaseExtensionRequest =
        LeaseExtensionRequest::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.cookie, b"cookiecookiecookiecookie",);
    test_write(&message, bytes);
}

#[test]
fn test_lease_extend_response_extended() {
    let bytes = include_bytes!("binary/svsc/lease_extend_response_extended.bin");
    let message: LeaseExtensionResponse =
        LeaseExtensionResponse::read(&mut Cursor::new(bytes)).unwrap();
    assert!(&message.new_expiration.is_some());
    test_write(&message, bytes);
}

#[test]
fn test_lease_extend_response_rejected() {
    let bytes = include_bytes!("binary/svsc/lease_response_rejected.bin");
    let message: LeaseExtensionResponse =
        LeaseExtensionResponse::read(&mut Cursor::new(bytes)).unwrap();
    assert!(&message.new_expiration.is_none());
    test_write(&message, bytes);
}

#[test]
fn test_establish_session_request() {
    let bytes = include_bytes!("binary/svsc/establish_session_request.bin");
    let message: EstablishSessionRequest =
        EstablishSessionRequest::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.lease_id, b"idid");
    test_write(&message, bytes);
}

#[test]
fn test_establish_session_response_success() {
    let bytes = include_bytes!("binary/svsc/establish_session_response_success.bin");
    let message: EstablishSessionResponse =
        EstablishSessionResponse::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.lease_id, b"idid");
    assert_eq!(&message.status, &EstablishSessionStatus::Success);
    assert!(&message.response_data.is_some());
    assert_eq!(
        &message.response_data.as_ref().unwrap().session_id,
        b"SESSIONIDSESSION"
    );
    assert_eq!(
        &message.response_data.as_ref().unwrap().peer_id,
        b"PEERIDPEERIDPEER"
    );
    assert_eq!(
        &message.response_data.as_ref().unwrap().peer_key,
        b"PEERKEYPEERKEYPE"
    );
    test_write(&message, bytes);
}

#[test]
#[should_panic]
fn test_establish_session_response_success_no_data() {
    let bytes = include_bytes!("binary/svsc/establish_session_response_success_no_data.bin");
    EstablishSessionResponse::read(&mut Cursor::new(bytes)).unwrap();
}

#[test]
fn test_establish_session_response_error() {
    let bytes = include_bytes!("binary/svsc/establish_session_response_error.bin");
    let message: EstablishSessionResponse =
        EstablishSessionResponse::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.lease_id, b"idid");
    assert_eq!(&message.status, &EstablishSessionStatus::IDNotFound);
    assert!(&message.response_data.is_none());
    test_write(&message, bytes);
}

#[test]
fn test_establish_session_notification() {
    let bytes = include_bytes!("binary/svsc/establish_session_notification.bin");
    let message: EstablishSessionNotification =
        EstablishSessionNotification::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.session_data.session_id, b"SESSIONIDSESSION");
    assert_eq!(&message.session_data.peer_id, b"PEERIDPEERIDPEER");
    assert_eq!(&message.session_data.peer_key, b"PEERKEYPEERKEYPE");
    test_write(&message, bytes);
}

#[test]
fn test_establish_session_end() {
    let bytes = &[0u8; 0];
    let message: SessionEnd = SessionEnd::read(&mut Cursor::new(bytes)).unwrap();
    test_write(&message, bytes);
}

#[test]
fn test_establish_session_end_notification() {
    let bytes = &[0u8; 0];
    let message: SessionEndNotification =
        SessionEndNotification::read(&mut Cursor::new(bytes)).unwrap();
    test_write(&message, bytes);
}

#[test]
fn test_session_data_send() {
    let bytes = include_bytes!("binary/svsc/session_data_send.bin");
    let message: SessionDataSend = SessionDataSend::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.data, b"YELLOW SUBMARINE");
    test_write(&message, bytes);
}

#[test]
fn test_session_data_receive() {
    let bytes = include_bytes!("binary/svsc/session_data_receive.bin");
    let message: SessionDataSend = SessionDataSend::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.data, b"YELLOW SUBMARINE");
    test_write(&message, bytes);
}

#[test]
fn test_keepalive() {
    let bytes = &[0u8; 0];
    let message: KeepAlive = KeepAlive::read(&mut Cursor::new(bytes)).unwrap();
    test_write(&message, bytes);
}
