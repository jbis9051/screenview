#[macro_use]
mod helper;
use crate::helper::*;
use common::messages::{sel::*, MessageComponent};
use std::io::Cursor;

#[test]
fn test_transport_data_message_reliable() {
    let bytes = include_bytes!("binary/sel/transport_data_message_reliable.bin");
    let message: TransportDataMessageReliable =
        TransportDataMessageReliable::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(message.data, b"test");
    test_write(&message, bytes);
}

#[test]
fn test_transport_data_peer_message_unreliable() {
    let bytes = include_bytes!("binary/sel/transport_data_peer_message_unreliable.bin");
    let message: TransportDataPeerMessageUnreliable =
        TransportDataPeerMessageUnreliable::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.peer_id, b"PEERIDPEERIDPEER");
    assert_eq!(&message.counter, b_to_u64!(b"COUNTERC"));
    assert_eq!(&message.data, b"test");
    test_write(&message, bytes);
}

#[test]
fn test_transport_data_server_message_unreliable() {
    let bytes = include_bytes!("binary/sel/transport_data_server_message_unreliable.bin");
    let message: TransportDataServerMessageUnreliable =
        TransportDataServerMessageUnreliable::read(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(&message.counter, b_to_u64!(b"COUNTERC"));
    assert_eq!(&message.data, b"test");
    test_write(&message, bytes);
}
