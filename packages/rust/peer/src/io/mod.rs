pub mod handle;
pub mod tcp;
pub mod udp;

pub use handle::*;
pub use tcp::*;
pub use udp::*;

pub(crate) const LENGTH_FIELD_WIDTH: usize = 2;
const INIT_BUFFER_CAPACITY: usize = 4096;
const UDP_READ_SIZE: usize = 65507;
const UDP_TIMEOUT: u64 = 50;
pub const DEFAULT_UNRELIABLE_MESSAGE_SIZE: usize = 1500;

#[macro_export]
macro_rules! return_if_err {
    ($expr:expr) => {
        if let Err(_) = $expr {
            return;
        }
    };
}

#[inline]
fn parse_length_field(message: &[u8]) -> usize {
    let mut length_bytes = [0u8; LENGTH_FIELD_WIDTH];
    length_bytes.copy_from_slice(&message[0 .. LENGTH_FIELD_WIDTH]);
    usize::from(u16::from_le_bytes(length_bytes))
}
