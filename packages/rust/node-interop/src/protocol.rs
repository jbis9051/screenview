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
    StartServer {
        addr: String,
    },
    EstablishSession {
        lease_id: [u8; 4],
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
    KeyboardInput {
        keycode: u32,
        down: bool,
    },
    LeaseRequest,
    UpdateStaticPassword {
        password: Option<String>,
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

pub struct Display {
    pub native_id: u32,
    pub display_type: DisplayType,
}

pub enum DisplayType {
    Monitor,
    Window,
}

impl TryFrom<&str> for DisplayType {
    type Error = InvalidEnumDiscriminant;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "monitor" => Ok(Self::Monitor),
            "window" => Ok(Self::Window),
            _ => Err(InvalidEnumDiscriminant),
        }
    }
}

#[derive(Debug)]
pub struct InvalidEnumDiscriminant;
