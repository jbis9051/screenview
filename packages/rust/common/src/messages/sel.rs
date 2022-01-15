use parser::{message_id, MessageComponent};

#[derive(Debug, MessageComponent)]
#[message_id(1)]
pub struct PeerHello {
    pub public_key: [u8; 16],
}

#[derive(Debug, MessageComponent)]
#[message_id(2)]
pub struct ServerHello {
    #[parse(len_prefixed(3))]
    pub certificate_list: Vec<u8>,
    pub public_key: [u8; 16],
    #[parse(greedy)]
    pub certificate_verify: Vec<u8>,
}

#[derive(Debug, MessageComponent)]
#[message_id(3)]
pub struct TransportDataMessageReliable {
    #[parse(greedy)]
    pub data: Vec<u8>,
}

#[derive(Debug, MessageComponent)]
#[message_id(4)]
pub struct TransportDataPeerMessageUnreliable {
    pub peer_id: [u8; 16],
    pub counter: u64,
    #[parse(greedy)]
    pub data: Vec<u8>,
}

#[derive(Debug, MessageComponent)]
#[message_id(5)]
pub struct TransportDataServerMessageUnreliable {
    pub counter: u64,
    #[parse(greedy)]
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum SelMessage {
    PeerHello(PeerHello),
    ServerHello(ServerHello),
    TransportDataMessageReliable(TransportDataMessageReliable),
    TransportDataPeerMessageUnreliable(TransportDataPeerMessageUnreliable),
    TransportDataServerMessageUnreliable(TransportDataServerMessageUnreliable),
}