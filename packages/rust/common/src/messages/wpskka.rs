use crate::messages::{Error, MessageComponent};
use byteorder::{ReadBytesExt, WriteBytesExt};
use parser::{message_id, MessageComponent};
use std::io::Cursor;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum AuthSchemeType {
    Invalid,
    SrpDynamic,
    SrpStatic,
    PublicKey,
}

impl TryFrom<u8> for AuthSchemeType {
    type Error = Error;

    fn try_from(auth_scheme_type: u8) -> Result<Self, Self::Error> {
        match auth_scheme_type {
            0 => Ok(Self::Invalid),
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

impl From<&AuthSchemeType> for u8 {
    fn from(data: &AuthSchemeType) -> Self {
        match data {
            AuthSchemeType::Invalid => 0,
            AuthSchemeType::SrpDynamic => 1,
            AuthSchemeType::SrpStatic => 2,
            AuthSchemeType::PublicKey => 3,
        }
    }
}

impl MessageComponent for AuthSchemeType {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let byte = cursor.read_u8()?;
        Self::try_from(byte)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(self.try_into()?).map_err(Into::into)
    }
}


#[derive(Debug, MessageComponent)]
#[message_id(1)]
pub struct AuthScheme {
    pub public_key: [u8; 16],
    #[parse(len_prefixed(1))]
    pub num_auth_schemes: Vec<AuthSchemeType>,
}

#[derive(Debug, MessageComponent)]
#[message_id(2)]
pub struct TryAuth {
    pub public_key: [u8; 16],
    pub auth_scheme: AuthSchemeType,
}

#[derive(Debug, MessageComponent)]
#[message_id(3)]
pub struct AuthMessage {
    #[parse(len_prefixed(2))]
    pub data: Vec<u8>,
}

#[derive(Debug, MessageComponent)]
#[message_id(4)]
pub struct AuthResult {
    pub ok: bool,
}

#[derive(Debug, MessageComponent)]
#[message_id(5)]
pub struct TransportDataMessageReliable {
    #[parse(len_prefixed(2))]
    pub data: Vec<u8>,
}

#[derive(Debug, MessageComponent)]
#[message_id(6)]
pub struct TransportDataMessageUnreliable {
    pub counter: u64,
    #[parse(len_prefixed(2))]
    pub data: Vec<u8>,
}

#[derive(MessageComponent, Debug)]
pub enum WpskkaMessage {
    AuthScheme(AuthScheme),
    TryAuth(TryAuth),
    AuthMessage(AuthMessage),
    AuthResult(AuthResult),
    TransportDataMessageReliable(TransportDataMessageReliable),
    TransportDataMessageUnreliable(TransportDataMessageUnreliable),
}
