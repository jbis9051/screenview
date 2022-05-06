use super::Data;
use parser::{message_id, MessageComponent};

#[derive(Debug, MessageComponent)]
#[message_id(1)]
#[lifetime('a)]
pub struct TransportDataMessageReliable<'a> {
    pub data: Data<'a>,
}

#[derive(Debug, MessageComponent)]
#[message_id(2)]
#[lifetime('a)]
pub struct TransportDataPeerMessageUnreliable<'a> {
    pub peer_id: [u8; 16],
    pub counter: u64,
    pub data: Data<'a>,
}

#[derive(Debug, MessageComponent)]
#[message_id(3)]
#[lifetime('a)]
pub struct TransportDataServerMessageUnreliable<'a> {
    pub counter: u64,
    pub data: Data<'a>,
}

#[derive(MessageComponent, Debug)]
#[lifetime('a)]
pub enum SelMessage<'a> {
    TransportDataMessageReliable(TransportDataMessageReliable<'a>),
    TransportDataPeerMessageUnreliable(TransportDataPeerMessageUnreliable<'a>),
    TransportDataServerMessageUnreliable(TransportDataServerMessageUnreliable<'a>),
}
