use byteorder::{LittleEndian, ReadBytesExt};
use common::messages::svsc::*;
use common::messages::MessageComponent;
use std::io::Cursor;

fn test_write<T: MessageComponent>(message: &T, bytes: &[u8]) {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    message.write(&mut cursor).unwrap();
    let inner = cursor.into_inner();
    assert_eq!(inner, bytes, "write failed");
}

#[test]
fn test_version() {
    let bytes = include_bytes!("binary/svsc/protocol_version.bin");
    let message: ProtocolVersion = ProtocolVersion::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.version, String::from_utf8(bytes.to_vec()).unwrap());
    test_write(&message, bytes);
}

#[test]
fn test_lease_request_cookie() {
    let bytes = include_bytes!("binary/svsc/lease_request_cookie.bin");
    let message: LeaseRequest = LeaseRequest::read(&mut Cursor::new(bytes)).unwrap();
    assert!(message.cookie.is_some());
    assert_eq!(&message.cookie.unwrap(), b"cookiecookiecookiecookie");
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
