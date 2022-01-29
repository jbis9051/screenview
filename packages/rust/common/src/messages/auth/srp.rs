use parser::{message_id, MessageComponent};

#[derive(Debug, MessageComponent)]
#[message_id(1)]
pub struct HostHello {
    pub username: [u8; 16],
    pub salt: [u8; 16],
    pub b_pub: [u8; 32],
}

#[derive(Debug, MessageComponent)]
#[message_id(2)]
pub struct ClientHello {
    pub a_pub: [u8; 32],
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
