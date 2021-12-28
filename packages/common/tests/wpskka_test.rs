mod helper;
use common::messages::wpskka::*;
use common::messages::MessageComponent;
use std::io::Cursor;
use crate::helper::test_write;


#[test]
fn test_host_hello() {
    let bytes = include_bytes!("binary/wpskka/host_hello.bin");
    let message: HostHello = HostHello::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.username, bytes[..16]);
    assert_eq!(message.salt, bytes[16..32]);
    assert_eq!(message.b_pub, bytes[32..288]);
    assert_eq!(message.public_key, bytes[288..]);
    test_write(&message, bytes);
}

#[test]
fn test_client_hello() {
    let bytes = include_bytes!("binary/wpskka/client_hello.bin");
    let message: ClientHello = ClientHello::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.a_pub, bytes[..256]);
    assert_eq!(message.public_key, bytes[256..272]);
    assert_eq!(message.mac, bytes[272..]);
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
fn test_transport_data() {
    let bytes = include_bytes!("binary/wpskka/transport_data.bin");
    let message: TransportDataMessage = TransportDataMessage::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.counter, b"COUNTERC");
    assert_eq!(&message.data, b"YELLOW SUBMARINE");
    test_write(&message, bytes);
}