use super::{Error, MessageComponent};
use parser::{message_id, MessageComponent};
use std::io::Cursor;

#[derive(MessageComponent)]
#[message_id(1)]
pub struct PeerHello {
    pub public_key: [u8; 16],
}

#[message_id(2)]
pub struct ServerHello {
    pub certificate_list: Vec<u8>,
    pub public_key: [u8; 16],
    pub certificate_verify: Vec<u8>,
}

impl MessageComponent for ServerHello {
    fn read(_cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        // Note: we assume that the type and total length have already been read
        todo!()
    }

    fn write(&self, _cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        // Note: we do not write the type nor total length
        todo!()
    }
}

#[derive(MessageComponent)]
#[message_id(3)]
pub struct TransportDataMessageReliable {
    #[parse(greedy)]
    pub data: Vec<u8>,
}

#[derive(MessageComponent)]
#[message_id(4)]
pub struct TransportDataPeerMessageUnreliable {
    pub peer_id: [u8; 16],
    pub counter: [u8; 8],
    #[parse(greedy)]
    pub data: Vec<u8>,
}

#[derive(MessageComponent)]
#[message_id(5)]
pub struct TransportDataServerMessageUnreliable {
    pub counter: [u8; 8],
    #[parse(greedy)]
    pub data: Vec<u8>,
}
