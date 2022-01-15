mod message_component;
pub use message_component::*;
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

pub enum ScreenViewMessage {
    SelMessage(sel::SelMessage),
    SvscMessage(svsc::SvscMessage),
    WpskkaMessage(wpskka::WpskkaMessage),
    RvdMessage(rvd::RvdMessage),
}
