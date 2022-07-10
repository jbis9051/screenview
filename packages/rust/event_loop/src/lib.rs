pub mod event_loop;
pub mod oneshot;

use std::thread::JoinHandle;

pub struct JoinOnDrop<T> {
    handle: Option<JoinHandle<T>>,
}

impl<T> JoinOnDrop<T> {
    pub fn new(handle: JoinHandle<T>) -> Self {
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
