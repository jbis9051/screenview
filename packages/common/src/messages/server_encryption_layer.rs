use parser::message_id;

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

#[message_id(3)]
pub struct TransportDataMessageReliable {
    pub data: Vec<u8>,
}

#[message_id(4)]
pub struct TransportDataPeerMessageUnreliable {
    pub peer_id: [u8; 16],
    pub counter: [u8; 8],
    pub data: Vec<u8>,
}

#[message_id(5)]
pub struct TransportDataServerMessageUnreliable {
    pub counter: [u8; 8],
    pub data: Vec<u8>,
}
