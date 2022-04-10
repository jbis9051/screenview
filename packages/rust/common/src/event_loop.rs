use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, Thread},
};

pub fn event_loop<F>(waker: ThreadWaker, mut func: F)
where F: FnMut() -> EventLoopState {
    loop {
        if func() == EventLoopState::Complete {
            return;
        }

        let unparked = waker.unparked.swap(false, Ordering::Acquire);
        if !unparked {
            thread::park();
            waker.unparked.store(false, Ordering::Release);
        }
    }
}

#[derive(Clone)]
pub struct ThreadWaker {
    thread: Thread,
    unparked: Arc<AtomicBool>,
}

impl ThreadWaker {
    pub fn new_current_thread() -> Self {
        Self {
            thread: thread::current(),
            unparked: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn wake(&self) {
        let unparked = self.unparked.swap(true, Ordering::Relaxed);
        if !unparked {
            self.thread.unpark();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EventLoopState {
    Complete,
    Working,
}
