#[macro_use]
mod helper;
use crate::helper::*;
use common::messages::{wpskka::*, MessageComponent};
use std::io::Cursor;

#[test]
fn test_host_hello() {
    let bytes = include_bytes!("binary/wpskka/host_hello.bin");
    let message: HostHello = HostHello::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.username, bytes[.. 16]);
    assert_eq!(message.salt, bytes[16 .. 32]);
    assert_eq!(message.b_pub, bytes[32 .. 64]);
    assert_eq!(message.public_key, bytes[64 ..]);
    test_write(&message, bytes);
}

#[test]
fn test_client_hello() {
    let bytes = include_bytes!("binary/wpskka/client_hello.bin");
    let message: ClientHello = ClientHello::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.a_pub, bytes[.. 32]);
    assert_eq!(message.public_key, bytes[32 .. 48]);
    assert_eq!(message.mac, bytes[48 ..]);
    test_write(&message, bytes);
}

#[test]
fn test_host_verify() {
    let bytes = include_bytes!("binary/wpskka/host_verify.bin");
    let message: HostVerify = HostVerify::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.mac, bytes);
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
