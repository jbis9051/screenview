use super::{Error, MessageComponent};
use bitflags::bitflags;
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

#[derive(MessageComponent)]
#[message_id(1)]
pub struct DisplayChange {
    pub clipboard_readable: bool,
    #[parse(len_prefixed(1))]
    pub display_information: Vec<DisplayInformation>,
}

type DisplayId = u8;

#[derive(MessageComponent)]
pub struct DisplayInformation {
    pub display_id: DisplayId,
    pub width: u16,
    pub height: u16,
    pub cell_width: u16,
    pub cell_height: u16,
    pub access: AccessMask,
    #[parse(len_prefixed(1))]
    pub name: String,
}

bitflags! {
    pub struct AccessMask: u8 {
        const FLUSH = 0b01;
        const CONTROLLABLE = 0b10;
    }
}

impl MessageComponent for AccessMask {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor
            .read_u8()
            .map(Self::from_bits_truncate)
            .map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()> {
        cursor.write_u8(self.bits)
    }
}

#[derive(MessageComponent)]
#[message_id(2)]
pub struct DisplayChangeReceived {}

#[derive(MessageComponent)]
#[message_id(3)]
pub struct MouseLocation {
    pub display_id: DisplayId,
    pub x_location: u16,
    pub y_location: u16,
}

#[derive(MessageComponent)]
#[message_id(4)]
pub struct MouseInput {
    pub display_id: DisplayId,
    pub x_location: u16,
    pub y_location: u16,
    pub buttons: ButtonsMask,
}

#[derive(MessageComponent)]
pub struct ButtonsMask {
    // TODO
}

#[derive(MessageComponent)]
#[message_id(5)]
pub struct KeyInput {
    pub down: bool,
    pub key: u16, // keysym
}

pub enum ClipboardType {
    Text,
    Rtf,
    Html,
    FilePointer,
    Custom(String)
}

#[derive(MessageComponent)]
#[message_id(6)]
pub struct ClipboardRequest {
    clipboard_type: ClipboardType
}

#[derive(MessageComponent)]
#[message_id(7)]
pub struct ClipboardResponse {
    clipboard_type: ClipboardType,
    content: Option<Vec<u8>>
}

#[derive(MessageComponent)]
#[message_id(8)]
pub struct FrameData {
    pub frame_number: u32,
    pub display_id: u8,
    pub cell_number: u16,
    #[parse(len_prefixed(2))]
    pub data: Vec<u8>,
}
