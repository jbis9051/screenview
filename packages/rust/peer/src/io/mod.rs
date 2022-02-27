mod handle;
mod tcp;
mod udp;

pub use handle::*;
use std::thread::JoinHandle;

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

struct JoinOnDrop<T> {
    handle: Option<JoinHandle<T>>,
}

impl<T> JoinOnDrop<T> {
    fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl<T> Drop for JoinOnDrop<T> {
    fn drop(&mut self) {
        // Drop can only ever be called once so unwrap will never fail
        drop(self.handle.take().unwrap().join());
    }
}
