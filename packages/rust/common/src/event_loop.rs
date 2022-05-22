use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle, Thread},
};

pub fn event_loop<F>(waker_core: ThreadWakerCore, mut func: F)
where F: FnMut(&ThreadWakerCore) -> EventLoopState {
    loop {
        match func(&waker_core) {
            EventLoopState::Working => waker_core.maybe_park(),
            EventLoopState::Complete => return,
        }
    }
}

pub struct ThreadWakerCore {
    state: Arc<AtomicUsize>,
    _not_send_sync: PhantomData<*const u8>,
}

impl ThreadWakerCore {
    const UNPARKED: usize = 1usize << (usize::BITS - 1);

    pub fn new_current_thread() -> Self {
        Self {
            state: Arc::new(AtomicUsize::new(0)),
            _not_send_sync: PhantomData,
        }
    }

    pub fn check_and_unset(&self, flag_bit: u32) -> bool {
        Self::check_flag_bit(flag_bit);

        let flag = 1usize << flag_bit;
        let old_state = self.state.fetch_and(!flag, Ordering::Relaxed);
        (old_state & flag) != 0
    }

    pub fn make_waker(&self, flag_bit: u32) -> ThreadWaker {
        Self::check_flag_bit(flag_bit);

        let state = Arc::clone(&self.state);
        let thread = thread::current();

        ThreadWaker {
            state,
            flag: (1usize << flag_bit) | Self::UNPARKED,
            thread,
        }
    }

    pub fn wake_self(&self, flag_bit: u32) {
        Self::check_flag_bit(flag_bit);

        self.state
            .fetch_or((1usize << flag_bit) | Self::UNPARKED, Ordering::Relaxed);
    }

    pub fn maybe_park(&self) {
        let old_state = self.state.fetch_and(!Self::UNPARKED, Ordering::Relaxed);

        if Self::is_parked(old_state) {
            // If we were spuriously unparked, then re-park
            thread::park();
            self.state.fetch_and(!Self::UNPARKED, Ordering::Relaxed);
        }
    }

    const fn is_parked(state: usize) -> bool {
        (state & Self::UNPARKED) == 0
    }

    fn check_flag_bit(flag_bit: u32) {
        debug_assert!(
            flag_bit < usize::BITS - 1,
            "flag cannot set the highest bit of the state"
        );
    }
}

#[derive(Clone)]
pub struct ThreadWaker {
    state: Arc<AtomicUsize>,
    flag: usize,
    thread: Thread,
}

impl ThreadWaker {
    pub fn wake(&self) {
        let old_state = self.state.fetch_or(self.flag, Ordering::Relaxed);

        if ThreadWakerCore::is_parked(old_state) {
            self.thread.unpark();
        }
    }
}

impl Drop for ThreadWaker {
    fn drop(&mut self) {
        self.wake();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EventLoopState {
    Working,
    Complete,
}

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
