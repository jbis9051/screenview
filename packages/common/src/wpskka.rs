// type = 1
pub struct HostHello {
    pub username: [u8; 16],
    pub salt: [u8; 16],
    pub b_pub: [u8; 256],
    pub public_key: [u8; 16],
}

// type = 2
pub struct ClientHello {
    pub username: [u8; 16],
    pub a_pub: [u8; 256],
    pub public_key: [u8; 16],
    pub mac: [u8; 32]
}

// type = 3
pub struct HostVerify {
    pub mac: [u8; 32]
}

// type = 4
pub struct TransportDataMessage {
    pub counter: [u8; 8],
    pub data: Vec<u8>,
}