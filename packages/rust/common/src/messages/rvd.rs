use super::{Error, MessageComponent};
use crate::messages::impl_bitflags_message_component;
use bitflags::bitflags;
use byteorder::{ReadBytesExt, WriteBytesExt};
use parser::{message_id, MessageComponent};
use std::{
    borrow::Cow,
    io::{Cursor, Read, Write},
};

#[derive(MessageComponent)]
pub struct ProtocolVersion {
    #[parse(fixed_len(11))]
    pub version: String, // fixed 11 bytes
}

#[derive(MessageComponent, Debug)]
pub struct ProtocolVersionResponse {
    pub ok: bool,
}

#[derive(MessageComponent, Debug)]
#[message_id(1)]
pub struct DisplayChange {
    pub clipboard_readable: bool,
    #[parse(len_prefixed(1))]
    pub display_information: Vec<DisplayInformation>,
}

type DisplayId = u8;

#[derive(MessageComponent, Debug)]
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

impl_bitflags_message_component!(AccessMask);

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

bitflags! {
    pub struct ButtonsMask: u8 {
        const LEFT          = 0b00000001;
        const MIDDLE        = 0b00000010;
        const RIGHT         = 0b00000100;
        const SCROLL_UP     = 0b00001000;
        const SCROLL_DOWN   = 0b00010000;
        const SCROLL_LEFT   = 0b00100000;
        const SCROLL_RIGHT  = 0b01000000;
    }
}

impl_bitflags_message_component!(ButtonsMask);

#[derive(MessageComponent)]
#[message_id(5)]
pub struct KeyInput {
    pub down: bool,
    pub key: u32, // keysym
}

#[derive(PartialEq, Debug)]
pub enum ClipboardType {
    Text,
    Rtf,
    Html,
    FilePointer,
    Custom(String),
}

impl TryFrom<u8> for ClipboardType {
    type Error = Error;

    fn try_from(clipboard_type: u8) -> Result<Self, Self::Error> {
        match clipboard_type {
            1 => Ok(Self::Text),
            2 => Ok(Self::Rtf),
            3 => Ok(Self::Html),
            4 => Ok(Self::FilePointer),
            _ => Err(Error::InvalidEnumValue {
                name: "ClipboardType",
                value: u16::from(clipboard_type),
            }),
        }
    }
}

impl From<&ClipboardType> for u8 {
    fn from(data: &ClipboardType) -> Self {
        match data {
            ClipboardType::Custom(_) => 0,
            ClipboardType::Text => 1,
            ClipboardType::Rtf => 2,
            ClipboardType::Html => 3,
            ClipboardType::FilePointer => 4,
        }
    }
}

struct ClipboardCustomName<'a>(pub Cow<'a, str>);

impl<'a> MessageComponent for ClipboardCustomName<'a> {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        // The length of this field takes up one byte
        let length = usize::from(cursor.read_u8()?);

        let mut utf8_bytes = vec![0u8; length];
        cursor.read_exact(&mut utf8_bytes)?;

        Ok(Self(Cow::Owned(String::from_utf8(utf8_bytes)?)))
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(u8::try_from(self.0.len())?)?;
        cursor.write_all(self.0.as_bytes()).map_err(Into::into)
    }
}

#[derive(MessageComponent)]
struct ClipboardMetaInter<'a> {
    clipboard_type: u8,
    #[parse(condition = "(clipboard_type & Self::CUSTOM_FLAG) != 0")]
    custom_name: Option<ClipboardCustomName<'a>>,
}

#[rustfmt::skip]
impl ClipboardMetaInter<'_> {
    const CUSTOM_FLAG: u8          = 0b10000000;
    const CONTENT_REQUEST_FLAG: u8 = 0b01000000;
    const DISCRIMINANT_MASK: u8    = 0b00111111;

    fn type_to_parts(&self) -> (bool, bool, u8) {
        let custom = (self.clipboard_type & Self::CUSTOM_FLAG) != 0;
        let content_request = (self.clipboard_type & Self::CONTENT_REQUEST_FLAG) != 0;
        let discriminant = self.clipboard_type & Self::DISCRIMINANT_MASK;

        (custom, content_request, discriminant)
    }
}

impl<'a> From<&'a ClipboardMeta> for ClipboardMetaInter<'a> {
    fn from(data: &'a ClipboardMeta) -> Self {
        let mut clipboard_type = 0u8;

        let custom_name = match &data.clipboard_type {
            ClipboardType::Custom(name) => {
                clipboard_type |= Self::CUSTOM_FLAG;
                Some(ClipboardCustomName(Cow::Borrowed(name.as_str())))
            }
            _ => None,
        };

        if data.content_request {
            clipboard_type |= Self::CONTENT_REQUEST_FLAG;
        }

        clipboard_type |= u8::from(&data.clipboard_type);

        Self {
            clipboard_type,
            custom_name,
        }
    }
}

impl TryFrom<ClipboardMetaInter<'_>> for ClipboardMeta {
    type Error = Error;

    fn try_from(data: ClipboardMetaInter<'_>) -> Result<Self, Self::Error> {
        let (custom, content_request, discriminant) = data.type_to_parts();

        if custom != (discriminant == 0) || custom != data.custom_name.is_some() {
            return Err(Error::InvalidEnumValue {
                name: "ClipboardType + Flags",
                value: u16::from(data.clipboard_type),
            });
        }

        Ok(match data.custom_name {
            Some(name) => Self {
                clipboard_type: ClipboardType::Custom(name.0.into_owned()),
                content_request,
            },
            None => Self {
                clipboard_type: ClipboardType::try_from(discriminant)?,
                content_request,
            },
        })
    }
}

pub struct ClipboardMeta {
    pub clipboard_type: ClipboardType,
    pub content_request: bool,
}

impl MessageComponent for ClipboardMeta {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        ClipboardMetaInter::read(cursor)?.try_into()
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        ClipboardMetaInter::<'_>::from(self).write(cursor)
    }
}

#[derive(MessageComponent)]
#[message_id(6)]
pub struct ClipboardRequest {
    pub info: ClipboardMeta,
}

#[derive(MessageComponent)]
#[message_id(7)]
pub struct ClipboardNotification {
    pub info: ClipboardMeta,
    #[parse(condition = "info.content_request", len_prefixed(3))]
    pub content: Option<Vec<u8>>,
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
