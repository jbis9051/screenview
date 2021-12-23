pub struct ProtocolVersion {
    pub version: String, // fixed 11 bytes
}

pub struct ProtocolVersionResponse {
    pub ok: bool,
}

pub type Cookie = [u8; 24];

// type = 1
pub struct LeaseRequest {
    pub has_cookie: bool,
    pub cookie: Option<Cookie>,
}

// type = 2
pub struct LeaseResponse {
    pub accepted: bool,
    pub response_data: Option<LeaseResponseData>,
}

pub type ExpirationTime = u64;

pub struct LeaseResponseData {
    pub id: u16,
    pub cookie: Cookie,
    pub expiration: ExpirationTime,
}

// type = 3
pub struct LeaseExtensionRequest {
    pub cookie: Cookie,
}

// type = 4
pub struct LeaseExtensionResponse {
    pub extended: bool,
    pub new_expiration: Option<ExpirationTime>,
}

pub type LeaseId = u32;

// type = 5
pub struct EstablishSessionRequest {
    pub lease_id: LeaseId,
}


pub enum EstablishSessionStatus {
    Success = 0x00,
    IDNotFound = 0x01,
    PeerOffline = 0x02,
    PeerBusy = 0x03,
    SelfBusy = 0x04,
    OtherError = 0x05,
}

// type = 6
pub struct EstablishSessionResponse {
    pub lease_id: u32,
    pub status: EstablishSessionStatus,
    pub response_data: Option<EstablishSessionResponseData>,
}

pub type SessionId = [u8; 16];
pub type PeerId = [u8; 16];
pub type PeerKey = [u8; 16];

pub struct EstablishSessionResponseData {
    pub session_id: SessionId,
    pub peer_id: PeerId,
    pub peer_key: PeerKey,
}

// type = 7
pub struct EstablishSessionNotification {
    pub session_id: SessionId,
    pub peer_id: PeerId,
    pub peer_key: PeerKey,
}

// type = 8
pub struct SessionEnd {}

// type = 9
pub struct SessionEndNotification {}

// type = 10
pub struct SessionDataSend {
    pub data: Vec<u8>,
}

// type = 11
pub struct SessionDataReceive {
    pub data: Vec<u8>,
}

// type = 0
pub struct KeepAlive {}