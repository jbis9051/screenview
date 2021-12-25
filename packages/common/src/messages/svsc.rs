use super::Error;
use super::MessageComponent;
use byteorder::{ReadBytesExt, WriteBytesExt};
use parser::{message_id, MessageComponent};
use std::io::{self, Cursor};

#[derive(MessageComponent)]
pub struct ProtocolVersion {
    #[parse(fixed_len(11))]
    pub version: String, // fixed 11 bytes
}

#[derive(MessageComponent)]
pub struct ProtocolVersionResponse {
    pub ok: bool,
}

pub type Cookie = [u8; 24];

#[derive(MessageComponent)]
#[message_id(1)]
pub struct LeaseRequest {
    pub has_cookie: bool,
    #[parse(condition = "has_cookie")]
    pub cookie: Option<Cookie>,
}

#[derive(MessageComponent)]
#[message_id(2)]
pub struct LeaseResponse {
    pub accepted: bool,
    #[parse(condition = "accepted")]
    pub response_data: Option<LeaseResponseData>,
}

pub type ExpirationTime = u64;

#[derive(MessageComponent)]
pub struct LeaseResponseData {
    pub id: u16,
    pub cookie: Cookie,
    pub expiration: ExpirationTime,
}

#[derive(MessageComponent)]
#[message_id(3)]
pub struct LeaseExtensionRequest {
    pub cookie: Cookie,
}

#[derive(MessageComponent)]
#[message_id(4)]
pub struct LeaseExtensionResponse {
    pub extended: bool,
    #[parse(condition = "extended")]
    pub new_expiration: Option<ExpirationTime>,
}

pub type LeaseId = u32;

#[derive(MessageComponent)]
#[message_id(5)]
pub struct EstablishSessionRequest {
    pub lease_id: LeaseId,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EstablishSessionStatus {
    Success = 0x00,
    IDNotFound = 0x01,
    PeerOffline = 0x02,
    PeerBusy = 0x03,
    SelfBusy = 0x04,
    OtherError = 0x05,
}

impl MessageComponent for EstablishSessionStatus {
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
                value: n as _,
            }),
        }
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()> {
        cursor.write_u8(*self as u8)
    }
}

#[derive(MessageComponent)]
#[message_id(6)]
pub struct EstablishSessionResponse {
    pub lease_id: u32,
    pub status: EstablishSessionStatus,
    #[parse(condition = "status == EstablishSessionStatus::Success")]
    pub response_data: Option<EstablishSessionResponseData>,
}

pub type SessionId = [u8; 16];
pub type PeerId = [u8; 16];
pub type PeerKey = [u8; 16];

#[derive(MessageComponent)]
pub struct EstablishSessionResponseData {
    pub session_id: SessionId,
    pub peer_id: PeerId,
    pub peer_key: PeerKey,
}

#[derive(MessageComponent)]
#[message_id(7)]
pub struct EstablishSessionNotification {
    pub session_id: SessionId,
    pub peer_id: PeerId,
    pub peer_key: PeerKey,
}

#[derive(MessageComponent)]
#[message_id(8)]
pub struct SessionEnd {}

#[derive(MessageComponent)]
#[message_id(9)]
pub struct SessionEndNotification {}

#[derive(MessageComponent)]
#[message_id(10)]
pub struct SessionDataSend {
    #[parse(len_prefixed(3))]
    pub data: Vec<u8>,
}

#[derive(MessageComponent)]
#[message_id(11)]
pub struct SessionDataReceive {
    #[parse(len_prefixed(3))]
    pub data: Vec<u8>,
}

#[derive(MessageComponent)]
#[message_id(0)]
pub struct KeepAlive {}
