#[macro_use]
mod helper;
use crate::helper::*;
use common::messages::{
    auth::srp::{ClientHello, HostHello, HostVerify},
    MessageComponent,
};
use std::io::Cursor;

#[test]
fn test_host_hello() {
    let bytes = include_bytes!("binary/wpskka/srp/host_hello.bin");
    let message: HostHello = HostHello::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.username, bytes[.. 16]);
    assert_eq!(message.salt, bytes[16 .. 32]);
    assert_eq!(*message.b_pub, bytes[32 ..]);
    test_write(&message, bytes);
}

#[test]
fn test_client_hello() {
    let bytes = include_bytes!("binary/wpskka/srp/client_hello.bin");
    let message: ClientHello = ClientHello::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(*message.a_pub, bytes[.. 256]);
    assert_eq!(message.mac, bytes[256 ..]);
    test_write(&message, bytes);
}

#[test]
fn test_host_verify() {
    let bytes = include_bytes!("binary/wpskka/srp/host_verify.bin");
    let message: HostVerify = HostVerify::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.mac, bytes);
    test_write(&message, bytes);
}
