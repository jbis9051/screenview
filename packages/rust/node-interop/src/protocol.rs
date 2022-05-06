use common::messages::rvd::ButtonsMask;
use neon::types::Deferred;
use std::convert::TryFrom;

pub enum Message {
    Request {
        content: RequestContent,
        promise: Deferred,
    },
    Shutdown,
}

impl Message {
    pub fn request(content: RequestContent, promise: Deferred) -> Self {
        Self::Request { content, promise }
    }
}

pub enum RequestContent {
    Connect {
        addr: String,
        connection_type: ConnectionType,
    },
    EstablishSession {
        lease_id: String,
    },
    ProcessPassword {
        password: String,
    },
    MouseInput {
        x_position: i32,
        y_position: i32,
        button_mask: ButtonsMask,
        button_mask_state: ButtonsMask,
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
