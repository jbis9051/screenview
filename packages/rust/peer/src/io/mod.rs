pub mod handle;
pub mod tcp;
pub mod udp;

pub use handle::*;
pub use tcp::*;
pub use udp::*;

const INIT_BUFFER_CAPACITY: usize = 4096;
const UDP_READ_SIZE: usize = 65507;
pub const DEFAULT_UNRELIABLE_MESSAGE_SIZE: usize = 1500;

#[macro_export]
macro_rules! return_if_err {
    ($expr:expr) => {
        if let Err(_) = $expr {
            return;
        }
    };
}
