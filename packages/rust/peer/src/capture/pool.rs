use super::FrameCapture;
use common::{event_loop::ThreadWaker, messages::rvd::DisplayId};
use native::NativeApiError;

pub struct CapturePool {
    captures: Vec<FrameCapture>,
    next_inactive: usize,
    waker: ThreadWaker,
}

impl CapturePool {
    pub fn new(waker: ThreadWaker) -> Self {
        Self {
            captures: Vec::new(),
            next_inactive: 0,
            waker,
        }
    }

    pub fn is_capturing(&self, display_id: DisplayId) -> bool {
        self.captures
            .iter()
            .any(|capture| capture.is_capturing(display_id))
    }

    pub fn get_or_create_inactive(&mut self) -> Result<&mut FrameCapture, NativeApiError> {
        let ret = if self.next_inactive >= self.captures.len() {
            self.captures.push(FrameCapture::new(self.waker.clone())?);
            Ok(self.captures.last_mut().unwrap())
        } else {
            let capture = &mut self.captures[self.next_inactive];
            Ok(capture)
        };

        self.next_inactive += 1;

        ret
    }

    pub fn active_captures(&mut self) -> impl Iterator<Item = &'_ mut FrameCapture> {
        self.captures.iter_mut().take(self.next_inactive)
    }
}
