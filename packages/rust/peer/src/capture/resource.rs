use native::api::Frame;

use super::{processing::FrameProcessorResources, FrameUpdate};
use common::messages::rvd::DisplayId;

pub struct CaptureResources {
    frame: Frame,
    processing: FrameProcessorResources,
}

impl CaptureResources {
    pub(super) fn new() -> Self {
        Self {
            frame: Frame::new(0, 0),
            processing: FrameProcessorResources::new(),
        }
    }

    pub(super) fn frame_mut(&mut self) -> &mut Frame {
        &mut self.frame
    }

    pub(super) fn frame_update(&self, display_id: DisplayId) -> FrameUpdate<'_> {
        self.processing.frame_update(&self.frame, display_id)
    }
}
