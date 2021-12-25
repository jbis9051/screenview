use parser::{message_id, MessageComponent};

#[derive(MessageComponent)]
#[message_id(1)]
pub struct HostHello {
    pub username: [u8; 16],
    pub salt: [u8; 16],
    #[parse(fixed_len(256))]
    pub b_pub: Vec<u8>,
    pub public_key: [u8; 16],
}

#[derive(MessageComponent)]
#[message_id(2)]
pub struct ClientHello {
    pub username: [u8; 16],
    pub a_pub: [u8; 256],
    pub public_key: [u8; 16],
    pub mac: [u8; 32],
}

#[derive(MessageComponent)]
#[message_id(3)]
pub struct HostVerify {
    pub mac: [u8; 32],
}

#[derive(MessageComponent)]
#[message_id(4)]
pub struct TransportDataMessage {
    pub counter: [u8; 8],
    #[parse(len_prefixed(2))]
    pub data: Vec<u8>,
}
