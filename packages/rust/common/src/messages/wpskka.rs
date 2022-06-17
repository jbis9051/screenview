use super::{Data, Message};
use crate::messages::{Error, MessageComponent};
use byteorder::{ReadBytesExt, WriteBytesExt};
use parser::{message_id, MessageComponent};
use std::io::Cursor;

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum AuthSchemeType {
    None,
    SrpDynamic,
    SrpStatic,
    PublicKey,
}

impl TryFrom<u8> for AuthSchemeType {
    type Error = Error;

    fn try_from(auth_scheme_type: u8) -> Result<Self, Self::Error> {
        match auth_scheme_type {
            0 => Ok(Self::None),
            1 => Ok(Self::SrpDynamic),
            2 => Ok(Self::SrpStatic),
            3 => Ok(Self::PublicKey),
            _ => Err(Error::InvalidEnumValue {
                name: "AuthSchemeType",
                value: u16::from(auth_scheme_type),
            }),
        }
    }
}

impl MessageComponent<'_> for AuthSchemeType {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let byte = cursor.read_u8()?;
        Self::try_from(byte)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(*self as u8).map_err(Into::into)
    }
}

#[derive(Debug, MessageComponent)]
#[message_id(1)]
pub struct KeyExchange {
    pub public_key: [u8; 32],
}

#[derive(Debug, MessageComponent)]
#[message_id(2)]
pub struct AuthScheme {
    #[parse(len_prefixed(1))]
    pub auth_schemes: Vec<AuthSchemeType>,
}

#[derive(Debug, MessageComponent)]
#[message_id(3)]
pub struct TryAuth {
    pub auth_scheme: AuthSchemeType,
}

#[derive(Debug, MessageComponent)]
#[message_id(4)]
pub struct AuthMessage {
    #[parse(len_prefixed(2))]
    pub data: Vec<u8>,
}

#[derive(Debug, MessageComponent)]
#[message_id(5)]
pub struct AuthResult {
    pub ok: bool,
}

#[derive(Debug, MessageComponent)]
#[message_id(6)]
#[lifetime('a)]
pub struct TransportDataMessageReliable<'a> {
    pub data: Data<'a>,
}

#[derive(Debug, MessageComponent)]
#[message_id(7)]
#[lifetime('a)]
pub struct TransportDataMessageUnreliable<'a> {
    pub counter: u64,
    pub data: Data<'a>,
}

#[derive(MessageComponent, Debug)]
#[lifetime('a)]
pub enum WpskkaMessage<'a> {
    KeyExchange(KeyExchange),
    AuthScheme(AuthScheme),
    TryAuth(TryAuth),
    AuthMessage(AuthMessage),
    AuthResult(AuthResult),
    TransportDataMessageReliable(TransportDataMessageReliable<'a>),
    TransportDataMessageUnreliable(TransportDataMessageUnreliable<'a>),
}

impl<'a> Message for WpskkaMessage<'a> {
    const LEN_PREFIX_WIDTH: usize = 2;
}
