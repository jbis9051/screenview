mod message_component;
pub use message_component::*;
pub mod auth;
pub mod rvd;
pub mod sel;
pub mod svsc;
pub mod wpskka;

macro_rules! impl_bitflags_message_component {
    ($name:ident) => {
        impl MessageComponent<'_> for $name {
            fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
                let flags = cursor.read_u8()?;
                Self::from_bits(flags).ok_or(Error::BadFlags {
                    name: stringify!($name),
                    value: flags,
                })
            }

            fn write(&self, cursor: &mut Cursor<Vec<u8>>) -> Result<(), Error> {
                cursor.write_u8(self.bits()).map_err(Into::into)
            }
        }
    };
}

pub(crate) use impl_bitflags_message_component;

use rvd::RvdMessage;
use sel::SelMessage;
use svsc::SvscMessage;
use wpskka::WpskkaMessage;

pub enum ScreenViewMessage<'a> {
    SelMessage(SelMessage<'a>),
    SvscMessage(SvscMessage<'a>),
    WpskkaMessage(WpskkaMessage<'a>),
    RvdMessage(RvdMessage),
}

impl<'a> From<SelMessage<'a>> for ScreenViewMessage<'a> {
    fn from(msg: SelMessage<'a>) -> Self {
        Self::SelMessage(msg)
    }
}

impl<'a> From<SvscMessage<'a>> for ScreenViewMessage<'a> {
    fn from(msg: SvscMessage<'a>) -> Self {
        Self::SvscMessage(msg)
    }
}

impl<'a> From<WpskkaMessage<'a>> for ScreenViewMessage<'a> {
    fn from(msg: WpskkaMessage<'a>) -> Self {
        Self::WpskkaMessage(msg)
    }
}

impl From<RvdMessage> for ScreenViewMessage<'_> {
    fn from(msg: RvdMessage) -> Self {
        Self::RvdMessage(msg)
    }
}
