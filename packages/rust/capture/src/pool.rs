use super::{processing::ProcessFrame, FrameCapture};
use common::messages::rvd::DisplayId;
use event_loop::event_loop::ThreadWaker;
use native::NativeApiError;

pub struct CapturePool<P: ProcessFrame> {
    captures: Vec<FrameCapture<P>>,
    next_inactive: usize,
    waker: ThreadWaker,
}

impl<P: ProcessFrame> CapturePool<P> {
    pub fn new(waker: ThreadWaker) -> Self {
        Self {
            captures: Vec::new(),
            next_inactive: 0,
            waker,
        }
    }

    pub fn deactivate(&mut self, display_id: DisplayId) {
        for capture in &mut self.captures {
            if capture.is_capturing(display_id) {
                capture.deactivate();
            }
        }
    }

    pub fn get_or_create_inactive(&mut self) -> Result<&mut FrameCapture<P>, NativeApiError> {
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

    pub fn active_captures(
        &mut self,
    ) -> impl Iterator<Item = (DisplayId, &'_ mut FrameCapture<P>)> {
        self.captures
            .iter_mut()
            .take(self.next_inactive)
            .map(|capture| (capture.captured_display(), capture))
    }
}
