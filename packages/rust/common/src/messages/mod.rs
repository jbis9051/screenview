mod message_component;
pub use message_component::*;
pub mod auth;
pub mod rvd;
pub mod sel;
pub mod svsc;
pub mod wpskka;

macro_rules! impl_bitflags_message_component {
    ($name:ident) => {
        impl MessageComponent for $name {
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

pub enum ScreenViewMessage {
    SelMessage(SelMessage),
    SvscMessage(SvscMessage),
    WpskkaMessage(WpskkaMessage),
    RvdMessage(RvdMessage),
}

impl From<SelMessage> for ScreenViewMessage {
    fn from(msg: SelMessage) -> Self {
        Self::SelMessage(msg)
    }
}

impl From<SvscMessage> for ScreenViewMessage {
    fn from(msg: SvscMessage) -> Self {
        Self::SvscMessage(msg)
    }
}

impl From<WpskkaMessage> for ScreenViewMessage {
    fn from(msg: WpskkaMessage) -> Self {
        Self::WpskkaMessage(msg)
    }
}

impl From<RvdMessage> for ScreenViewMessage {
    fn from(msg: RvdMessage) -> Self {
        Self::RvdMessage(msg)
    }
}
