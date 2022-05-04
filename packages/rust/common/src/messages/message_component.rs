use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    borrow::Cow,
    convert::Infallible,
    io::{self, Cursor, Read, Write},
    num::TryFromIntError,
    string::FromUtf8Error,
};

pub trait MessageID {
    const ID: u8;
}

pub trait MessageComponent<'a>: Sized {
    fn read(cursor: &mut Cursor<&'a [u8]>) -> Result<Self, Error>;

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error>;

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(Vec::new());
        self.write(&mut cursor)?;
        Ok(cursor.into_inner())
    }
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
    #[error("encountered bad or malformed SEL message")]
    BadSelMessage,
    #[error("data cursor reached an invalid state (position > data len)")]
    BadCursorState,
    #[error("encountered invalid data length")]
    BadDataLength,
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl<'a, T: MessageComponent<'a>> MessageComponent<'a> for Box<T> {
    fn read(cursor: &mut Cursor<&'a [u8]>) -> Result<Self, Error> {
        T::read(cursor).map(Box::new)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        T::write(&**self, cursor)
    }
}

impl MessageComponent<'_> for bool {
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

impl MessageComponent<'_> for u8 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u8().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u8(*self).map_err(Into::into)
    }
}

impl MessageComponent<'_> for u16 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u16::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u16::<LittleEndian>(*self).map_err(Into::into)
    }
}

impl<const N: usize> MessageComponent<'_> for [u8; N] {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let mut dest = [0u8; N];
        cursor.read_exact(&mut dest)?;
        Ok(dest)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_all(self.as_slice()).map_err(Into::into)
    }
}

impl MessageComponent<'_> for u32 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u32::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u32::<LittleEndian>(*self).map_err(Into::into)
    }
}

impl MessageComponent<'_> for u64 {
    fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        cursor.read_u64::<LittleEndian>().map_err(Into::into)
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        cursor.write_u64::<LittleEndian>(*self).map_err(Into::into)
    }
}

#[derive(Clone, Debug)]
pub struct Data<'a, const LEN_WIDTH: usize>(pub Cow<'a, [u8]>);

impl<'a, const LEN_WIDTH: usize> MessageComponent<'a> for Data<'a, LEN_WIDTH> {
    fn read(cursor: &mut Cursor<&'a [u8]>) -> Result<Self, Error> {
        let bytes = *cursor.get_ref();
        let mut position = usize::try_from(cursor.position())?;

        let len = match LEN_WIDTH {
            0 => bytes
                .len()
                .checked_sub(position)
                .ok_or(Error::BadCursorState)?,
            1 => usize::from(cursor.read_u8()?),
            2 => usize::from(cursor.read_u16::<LittleEndian>()?),
            3 => usize::try_from(cursor.read_u24::<LittleEndian>()?)?,
            4 => usize::try_from(cursor.read_u32::<LittleEndian>()?)?,
            _ => panic!("Invalid LEN_WIDTH in Data type: LEN_WIDTH cannot exceed 4 bytes"),
        };

        position = position
            .checked_add(LEN_WIDTH)
            .ok_or(Error::BadDataLength)?;

        let data_end = position.checked_add(len).ok_or(Error::BadDataLength)?;
        let data = bytes
            .get(position .. data_end)
            .ok_or(Error::BadDataLength)?;

        let new_position = u64::try_from(data_end)?;
        cursor.set_position(new_position);

        Ok(Self(Cow::Borrowed(data)))
    }

    fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
        let bytes = &*self.0;
        let len = bytes.len().to_le_bytes();
        cursor.write_all(&len[.. LEN_WIDTH])?;
        cursor.write_all(bytes).map_err(Into::into)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok((*self.0).to_owned())
    }
}
