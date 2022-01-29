#[macro_use]
mod helper;
use crate::helper::*;
use common::messages::{wpskka::*, MessageComponent};
use std::io::Cursor;

#[test]
fn test_auth_scheme() {
    let bytes = include_bytes!("binary/wpskka/auth_scheme.bin");
    let message: AuthScheme = AuthScheme::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.public_key, bytes[0 .. 16]);
    assert_eq!(message.num_auth_schemes.len(), 1);
    assert_eq!(message.num_auth_schemes[0], AuthSchemeType::SrpDynamic);
    test_write(&message, bytes);
}

#[test]
fn test_try_auth() {
    let bytes = include_bytes!("binary/wpskka/try_auth.bin");
    let message: TryAuth = TryAuth::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.public_key, bytes[0 .. 16]);
    assert_eq!(message.auth_scheme, AuthSchemeType::SrpStatic);
    test_write(&message, bytes);
}

#[test]
fn test_auth_message() {
    let bytes = include_bytes!("binary/wpskka/auth_message.bin");
    let message: AuthMessage = AuthMessage::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.data[..], bytes[2 ..]);
    test_write(&message, bytes);
}

#[test]
fn test_auth_result() {
    let bytes = include_bytes!("binary/wpskka/auth_result.bin");
    let message: AuthResult = AuthResult::read(&mut Cursor::new(bytes)).unwrap();
    assert!(&message.ok);
    test_write(&message, bytes);
}

#[test]
fn test_transport_data_unreliable() {
    let bytes = include_bytes!("binary/wpskka/transport_data_unreliable.bin");
    let message: TransportDataMessageUnreliable =
        TransportDataMessageUnreliable::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.counter, b_to_u64!(b"COUNTERC"));
    assert_eq!(&message.data, b"YELLOW SUBMARINE");
    test_write(&message, bytes);
}

#[test]
fn test_transport_data_reliable() {
    let bytes = include_bytes!("binary/wpskka/transport_data_reliable.bin");
    let message: TransportDataMessageReliable =
        TransportDataMessageReliable::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.data, b"YELLOW SUBMARINE");
    test_write(&message, bytes);
}
