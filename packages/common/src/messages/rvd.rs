use super::{Error, MessageComponent};
use crate::messages::Error::{InvalidEnumValue, InvalidString, StdIo};
use bitflags::bitflags;
use byteorder::{ReadBytesExt, WriteBytesExt};
use parser::{message_id, MessageComponent};
use std::io::{self, Cursor, Read, Write};

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

impl MessageComponent for AccessMask {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor
            .read_u8()
            .map(Self::from_bits_truncate)
            .map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(self.bits).map_err(Into::into)
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
bitflags! {
    pub struct ButtonsMask: u8 {
        // TODO
    }
}

impl MessageComponent for ButtonsMask {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor
            .read_u8()
            .map(Self::from_bits_truncate)
            .map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(self.bits).map_err(Into::into)
    }
}

#[derive(MessageComponent)]
#[message_id(5)]
pub struct KeyInput {
    pub down: bool,
    pub key: u32, // keysym
}

bitflags! {
    struct ClipboardTypeMask: u8 {
        const CUSTOM = 0b10000000;
        const CONTENT = 0b01000000;
    }
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
    type Error = ();

    fn try_from(clipboard_type: u8) -> Result<Self, Self::Error> {
        match clipboard_type {
            1 => Ok(Self::Text),
            2 => Ok(Self::Rtf),
            3 => Ok(Self::Html),
            4 => Ok(Self::FilePointer),
            _ => Err(()),
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

struct ClipboardMetaInter {
    clipboard_type: ClipboardTypeMask,
    custom_name: Option<String>,
}

impl MessageComponent for ClipboardMetaInter {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let clipboard_type = cursor
            .read_u8()
            .map(|u| unsafe { ClipboardTypeMask::from_bits_unchecked(u) })
            .unwrap();
        if clipboard_type.contains(ClipboardTypeMask::CUSTOM) {
            let length = cursor.read_u8().unwrap();
            let mut name = vec![0u8; length as usize];
            cursor.read_exact(&mut name).map_err(StdIo)?;
            let name = String::from_utf8(name).map_err(InvalidString)?;
            return Ok(ClipboardMetaInter {
                clipboard_type,
                custom_name: Some(name),
            });
        }
        Ok(ClipboardMetaInter {
            clipboard_type,
            custom_name: None,
        })
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        if self.custom_name.is_some() != self.clipboard_type.contains(ClipboardTypeMask::CUSTOM) {
            // enforce precondition
            return Err(io::Error::from(io::ErrorKind::InvalidData).into());
        }
        cursor.write_u8(self.clipboard_type.bits)?;
        if let Some(name) = &self.custom_name {
            let length: u8 = name
                .len()
                .try_into()
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
            cursor.write_u8(length)?;
            cursor.write_all(name.as_bytes())?;
        }
        Ok(())
    }
}

impl From<&ClipboardMeta> for ClipboardMetaInter {
    fn from(data: &ClipboardMeta) -> Self {
        let mut mask =
            unsafe { ClipboardTypeMask::from_bits_unchecked((&data.clipboard_type).into()) };
        mask.set(ClipboardTypeMask::CONTENT, data.content_request);
        let name = match &data.clipboard_type {
            ClipboardType::Custom(name) => {
                mask.set(ClipboardTypeMask::CUSTOM, true);
                Some(name.clone())
            }
            _ => None,
        };
        Self {
            clipboard_type: mask,
            custom_name: name,
        }
    }
}

pub struct ClipboardMeta {
    pub clipboard_type: ClipboardType,
    pub content_request: bool,
}

impl TryFrom<ClipboardMetaInter> for ClipboardMeta {
    type Error = Error;

    fn try_from(data: ClipboardMetaInter) -> Result<Self, Self::Error> {
        let content_request = data.clipboard_type.contains(ClipboardTypeMask::CONTENT);
        let clipboard_type = {
            let mut clip_type = data.clipboard_type;
            clip_type.set(ClipboardTypeMask::CUSTOM, false);
            clip_type.set(ClipboardTypeMask::CONTENT, false);
            clip_type.bits
        };
        Ok(match data.custom_name {
            Some(name) => {
                if clipboard_type != 0 {
                    return Err(Error::InvalidEnumValue { name: "ClipboardType", value: clipboard_type as u16 });
                }
                Self {
                    clipboard_type: ClipboardType::Custom(name),
                    content_request,
                }
            }
            None => {
                Self {
                    clipboard_type: ClipboardType::try_from(clipboard_type)
                        .map_err(|_| InvalidEnumValue { name: "ClipboardType", value: clipboard_type as u16 })?,
                    content_request,
                }
            }
        })
    }
}

impl MessageComponent for ClipboardMeta {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        ClipboardMetaInter::read(cursor)?.try_into()
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        let inter: ClipboardMetaInter = self.into();
        inter.write(cursor)
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
    #[parse(condition = "info.content_request" len_prefixed(3))]
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
