mod helper;
use crate::helper::test_write;
use common::messages::tel::*;
use common::messages::MessageComponent;
use std::io::Cursor;

#[test]
fn test_peer_hello() {
    let bytes = include_bytes!("binary/tel/peer_hello.bin");
    let message: PeerHello = PeerHello::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.public_key, b"PUBLICKEYPUBLICK");
    test_write(&message, bytes);
}

#[test]
fn test_server_hello() {
    let bytes = include_bytes!("binary/tel/server_hello.bin");
    let message: ServerHello = ServerHello::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.certificate_list, b"cert_list");
    assert_eq!(&message.public_key, b"PUBLICKEYPUBLICK");
    assert_eq!(&message.certificate_verify, b"verify");
    test_write(&message, bytes);
}

#[test]
fn test_transport_data_message_reliable() {
    let bytes = include_bytes!("binary/tel/transport_data_message_reliable.bin");
    let message: TransportDataMessageReliable =
        TransportDataMessageReliable::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.data, b"test");
    test_write(&message, bytes);
}

#[test]
fn test_transport_data_peer_message_unreliable() {
    let bytes = include_bytes!("binary/tel/transport_data_peer_message_unreliable.bin");
    let message: TransportDataPeerMessageUnreliable =
        TransportDataPeerMessageUnreliable::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.peer_id, b"PEERIDPEERIDPEER");
    assert_eq!(&message.counter, b"COUNTERC");
    assert_eq!(&message.data, b"test");
    test_write(&message, bytes);
}

#[test]
fn test_transport_data_server_message_unreliable() {
    let bytes = include_bytes!("binary/tel/transport_data_server_message_unreliable.bin");
    let message: TransportDataServerMessageUnreliable =
        TransportDataServerMessageUnreliable::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.counter, b"COUNTERC");
    assert_eq!(&message.data, b"test");
    test_write(&message, bytes);
}
