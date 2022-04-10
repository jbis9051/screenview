use neon::{event::Channel, types::Deferred};
use std::convert::TryFrom;

pub enum Message {
    Request {
        content: RequestContent,
        promise: PromiseHandle,
    },
    Shutdown,
}

impl Message {
    pub fn request(content: RequestContent, deferred: Deferred, channel: Channel) -> Self {
        Self::Request {
            content,
            promise: PromiseHandle { deferred, channel },
        }
    }
}

pub struct PromiseHandle {
    pub deferred: Deferred,
    pub channel: Channel,
}

pub enum RequestContent {
    Connect {
        addr: String,
        connection_type: ConnectionType,
    },
}

#[repr(u8)]
pub enum ConnectionType {
    Reliable,
    Unreliable,
}

impl TryFrom<u8> for ConnectionType {
    type Error = EnumDiscriminantOutOfRange;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Reliable),
            1 => Ok(Self::Unreliable),
            _ => Err(EnumDiscriminantOutOfRange),
        }
    }
}

pub struct EnumDiscriminantOutOfRange;
