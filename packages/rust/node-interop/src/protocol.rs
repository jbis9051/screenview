use common::messages::{
    rvd::ButtonsMask,
    svsc::{Cookie, LeaseId},
};
use neon::types::Deferred;
use peer::rvd::Display;
use std::{convert::TryFrom, fmt::Debug};

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
    StartServer {
        addr: String,
    },
    EstablishSession {
        lease_id: LeaseId,
    },
    ProcessPassword {
        password: Vec<u8>,
    },
    MouseInput {
        x_position: i32,
        y_position: i32,
        button_mask: ButtonsMask,
        button_mask_state: ButtonsMask,
    },
    KeyboardInput {
        keycode: u32,
        down: bool,
    },
    LeaseRequest {
        cookie: Option<Cookie>,
    },
    UpdateStaticPassword {
        password: Option<Vec<u8>>,
    },
    SetControllable {
        is_controllable: bool,
    },
    SetClipboardReadable {
        is_readable: bool,
    },
    ShareDisplays {
        displays: Vec<Display>,
    },
    NativeThumbnails,
}

#[repr(u8)]
pub enum ConnectionType {
    Reliable,
    Unreliable,
}

impl TryFrom<u8> for ConnectionType {
    type Error = InvalidEnumDiscriminant;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Reliable),
            1 => Ok(Self::Unreliable),
            _ => Err(InvalidEnumDiscriminant),
        }
    }
}

#[derive(Debug)]
pub struct InvalidEnumDiscriminant;
