use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::string::FromUtf8Error;
use std::{
    convert::Infallible,
    io::{self, Cursor, Read, Write},
    num::TryFromIntError,
};

pub trait MessageID {
    const ID: u8;
}

pub trait MessageComponent: Sized {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error>;

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    StdIo(#[from] io::Error),
    #[error("invalid string: {0}")]
    InvalidString(#[from] FromUtf8Error),
    #[error("encountered invalid enum value for enum {name}: {value}")]
    InvalidEnumValue { name: &'static str, value: u16 },
    #[error("encountered a length parameter too long to fit in a usize")]
    LengthTooLong(#[from] TryFromIntError),
    #[error("invalid date: {0}")]
    InvalidDate(i64),
    #[error("encountered bad boolean with value {0}")]
    BadBool(u8),
    #[error("encountered bad flags for {name} with value {value}")]
    BadFlags { name: &'static str, value: u8 },
    #[error("encountered invalid message id {0}")]
    BadMessageID(u8),
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl MessageComponent for bool {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        match cursor.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            by => Err(Error::BadBool(by)),
        }
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(*self as u8).map_err(Into::into)
    }
}

impl MessageComponent for u8 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u8().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(*self).map_err(Into::into)
    }
}

impl MessageComponent for u16 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u16::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u16::<LittleEndian>(*self).map_err(Into::into)
    }
}

impl<const N: usize> MessageComponent for [u8; N] {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let mut dest = [0u8; N];
        cursor.read_exact(&mut dest)?;
        Ok(dest)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_all(self.as_slice()).map_err(Into::into)
    }
}

impl MessageComponent for u32 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u32::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u32::<LittleEndian>(*self).map_err(Into::into)
    }
}

impl MessageComponent for u64 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u64::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u64::<LittleEndian>(*self).map_err(Into::into)
    }
}
