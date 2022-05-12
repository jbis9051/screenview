use parser::{message_id, MessageComponent};

use crate::messages::Message;

#[derive(Debug, MessageComponent)]
#[message_id(1)]
pub struct HostHello {
    pub username: [u8; 16],
    pub salt: [u8; 16],
    pub b_pub: Box<[u8; 256]>,
}

#[derive(Debug, MessageComponent)]
#[message_id(2)]
pub struct ClientHello {
    pub a_pub: Box<[u8; 256]>,
    pub mac: [u8; 32],
}

#[derive(Debug, MessageComponent)]
#[message_id(3)]
pub struct HostVerify {
    pub mac: [u8; 32],
}

#[derive(Debug, MessageComponent)]
pub enum SrpMessage {
    HostHello(HostHello),
    ClientHello(ClientHello),
    HostVerify(HostVerify),
}

impl Message for SrpMessage {
    const LEN_PREFIX_WIDTH: usize = 0;
}
