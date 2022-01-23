use parser::{message_id, MessageComponent};

#[derive(Debug, MessageComponent)]
#[message_id(1)]
pub struct TransportDataMessageReliable {
    #[parse(greedy)]
    pub data: Vec<u8>,
}

#[derive(Debug, MessageComponent)]
#[message_id(2)]
pub struct TransportDataPeerMessageUnreliable {
    pub peer_id: [u8; 16],
    pub counter: u64,
    #[parse(greedy)]
    pub data: Vec<u8>,
}

#[derive(Debug, MessageComponent)]
#[message_id(3)]
pub struct TransportDataServerMessageUnreliable {
    pub counter: u64,
    #[parse(greedy)]
    pub data: Vec<u8>,
}

#[derive(MessageComponent, Debug)]
pub enum SelMessage {
    TransportDataMessageReliable(TransportDataMessageReliable),
    TransportDataPeerMessageUnreliable(TransportDataPeerMessageUnreliable),
    TransportDataServerMessageUnreliable(TransportDataServerMessageUnreliable),
}
