use native::api::Frame;

use super::processing::{ProcessFrame, ProcessorResources};
use common::messages::rvd::DisplayId;

pub struct CaptureResources<P: ProcessFrame> {
    frame: Frame,
    processing: P::Resources,
}

impl<P: ProcessFrame> CaptureResources<P> {
    pub(super) fn new() -> Self {
        Self {
            frame: Frame::new(0, 0),
            processing: <P::Resources as Default>::default(),
        }
    }

    pub(super) fn frame_mut(&mut self) -> &mut Frame {
        &mut self.frame
    }

    pub(super) fn frame_update(
        &self,
        display_id: DisplayId,
    ) -> <P::Resources as ProcessorResources<'_>>::FrameUpdate {
        self.processing.to_frame_update(&self.frame, display_id)
    }
}
