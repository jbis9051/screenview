// type = 1
pub struct PeerHello {
    pub public_key: [u8; 16],
}

// type = 2
pub struct ServerHello {
    pub certificate_list: Vec<u8>,
    pub public_key: [u8; 16],
    pub certificate_verify: Vec<u8>,
}

// type = 3
pub struct TransportDataMessageReliable {
    pub data: Vec<u8>,
}

// type = 4
pub struct TransportDataPeerMessageUnreliable {
    pub peer_id: [u8; 16],
    pub counter: [u8; 8],
    pub data: Vec<u8>,
}

// type = 5
pub struct TransportDataServerMessageUnreliable {
    pub counter: [u8; 8],
    pub data: Vec<u8>,
}