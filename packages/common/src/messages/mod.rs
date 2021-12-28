use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::string::FromUtf8Error;
use std::{
    convert::Infallible,
    io::{self, Cursor, Read, Write},
    num::TryFromIntError,
};

pub mod rvd;
pub mod server_encryption_layer;
pub mod svsc;
pub mod wpskka;

pub trait MessageID {
    const ID: u8;
}

pub trait MessageComponent: Sized {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error>;

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()>;
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
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl MessageComponent for bool {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        Ok(cursor.read_u8()? == 1)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()> {
        cursor.write_u8(*self as u8)
    }
}

impl MessageComponent for u8 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u8().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()> {
        cursor.write_u8(*self)
    }
}

impl MessageComponent for u16 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u16::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()> {
        cursor.write_u16::<LittleEndian>(*self)
    }
}

impl<const N: usize> MessageComponent for [u8; N] {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let mut dest = [0u8; N];
        cursor.read_exact(&mut dest)?;
        Ok(dest)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()> {
        cursor.write_all(self.as_slice())
    }
}

impl MessageComponent for u32 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u32::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()> {
        cursor.write_u32::<LittleEndian>(*self)
    }
}

impl MessageComponent for u64 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u64::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> io::Result<()> {
        cursor.write_u64::<LittleEndian>(*self)
    }
}
