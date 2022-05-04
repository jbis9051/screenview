use super::{Data, Error, MessageComponent};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, LocalResult, TimeZone, Utc};
use parser::{message_id, MessageComponent};
use std::io::Cursor;

#[derive(Debug, MessageComponent)]
#[message_id(0)]
pub struct ProtocolVersion {
    #[parse(fixed_len(12))]
    pub version: String,
}

#[derive(Debug, MessageComponent)]
#[message_id(1)]
pub struct ProtocolVersionResponse {
    pub ok: bool,
}

pub type Cookie = [u8; 24];

#[derive(Debug, MessageComponent)]
#[message_id(2)]
pub struct LeaseRequest {
    #[parse(bool_prefixed)]
    pub cookie: Option<Cookie>,
}

#[derive(Debug, MessageComponent)]
#[message_id(3)]
pub struct LeaseResponse {
    #[parse(bool_prefixed)]
    pub response_data: Option<LeaseResponseData>,
}

pub type ExpirationTime = DateTime<Utc>;

impl MessageComponent<'_> for ExpirationTime {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let date = cursor.read_i64::<LittleEndian>()?;
        match Utc.timestamp_opt(date, 0) {
            LocalResult::Single(time) => Ok(time),
            _ => Err(Error::InvalidDate(date)),
        }
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor
            .write_i64::<LittleEndian>(self.timestamp())
            .map_err(Into::into)
    }
}

#[derive(Debug, MessageComponent, PartialEq, Clone)]
pub struct LeaseResponseData {
    pub id: u32,
    pub cookie: Cookie,
    pub expiration: ExpirationTime,
}

#[derive(Debug, MessageComponent)]
#[message_id(4)]
pub struct LeaseExtensionRequest {
    pub cookie: Cookie,
}

#[derive(Debug, MessageComponent)]
#[message_id(5)]
pub struct LeaseExtensionResponse {
    #[parse(bool_prefixed)]
    pub new_expiration: Option<ExpirationTime>,
}

pub type LeaseId = [u8; 4];

#[derive(Debug, MessageComponent)]
#[message_id(6)]
pub struct EstablishSessionRequest {
    pub lease_id: LeaseId,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum EstablishSessionStatus {
    Success = 0x00,
    IDNotFound = 0x01,
    PeerOffline = 0x02,
    PeerBusy = 0x03,
    SelfBusy = 0x04,
    OtherError = 0x05,
}

impl MessageComponent<'_> for EstablishSessionStatus {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        match cursor.read_u8()? {
            0 => Ok(Self::Success),
            1 => Ok(Self::IDNotFound),
            2 => Ok(Self::PeerOffline),
            3 => Ok(Self::PeerBusy),
            4 => Ok(Self::SelfBusy),
            5 => Ok(Self::OtherError),
            n => Err(Error::InvalidEnumValue {
                name: "EstablishSessionStatus",
                value: u16::from(n),
            }),
        }
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(*self as u8).map_err(Into::into)
    }
}

pub type SessionId = [u8; 16];
pub type PeerId = [u8; 16];
pub type PeerKey = [u8; 16];

#[derive(Debug, MessageComponent, Clone, PartialEq)]
pub struct SessionData {
    pub session_id: SessionId,
    pub peer_id: PeerId,
    pub peer_key: PeerKey,
}

#[derive(Debug, MessageComponent, Clone)]
#[message_id(7)]
pub struct EstablishSessionResponse {
    pub lease_id: LeaseId,
    pub status: EstablishSessionStatus,
    #[parse(condition = "status == EstablishSessionStatus::Success")]
    pub response_data: Option<Box<SessionData>>,
}

#[derive(Debug, MessageComponent)]
#[message_id(8)]
pub struct EstablishSessionNotification {
    pub session_data: SessionData,
}

#[derive(Debug, MessageComponent)]
#[message_id(9)]
pub struct SessionEnd {}

#[derive(Debug, MessageComponent)]
#[message_id(10)]
pub struct SessionEndNotification {}

#[derive(Debug, MessageComponent)]
#[message_id(11)]
#[lifetime('a)]
pub struct SessionDataSend<'a> {
    // #[parse(len_prefixed(3))]
    pub data: Data<'a, 3>,
}

#[derive(Debug, MessageComponent)]
#[message_id(12)]
#[lifetime('a)]
pub struct SessionDataReceive<'a> {
    // #[parse(len_prefixed(3))]
    pub data: Data<'a, 3>,
}

#[derive(Debug, MessageComponent)]
#[message_id(13)]
pub struct KeepAlive {}

#[derive(MessageComponent, Debug)]
#[lifetime('a)]
pub enum SvscMessage<'a> {
    ProtocolVersion(ProtocolVersion),
    ProtocolVersionResponse(ProtocolVersionResponse),
    LeaseRequest(LeaseRequest),
    LeaseResponse(LeaseResponse),
    LeaseExtensionRequest(LeaseExtensionRequest),
    LeaseExtensionResponse(LeaseExtensionResponse),
    EstablishSessionRequest(EstablishSessionRequest),
    EstablishSessionResponse(EstablishSessionResponse),
    EstablishSessionNotification(EstablishSessionNotification),
    SessionEnd(SessionEnd),
    SessionEndNotification(SessionEndNotification),
    SessionDataSend(SessionDataSend<'a>),
    SessionDataReceive(SessionDataReceive<'a>),
    KeepAlive(KeepAlive),
}
