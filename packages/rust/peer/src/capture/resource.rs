use native::api::Frame;

use super::{processing::ProcessFrame, ViewResources};
use common::messages::rvd::DisplayId;

pub struct CaptureResources<P: ProcessFrame> {
    pub(super) frame: Frame,
    pub(super) processing: P::Resources,
}

impl<P: ProcessFrame> CaptureResources<P> {
    pub(super) fn new() -> Self {
        Self {
            frame: Frame::new(0, 0),
            processing: <P::Resources as Default>::default(),
        }
    }
}

impl<P> CaptureResources<P>
where
    P: ProcessFrame,
    P: for<'a> ViewResources<'a, Resources = <P as ProcessFrame>::Resources>,
{
    pub(super) fn frame_update(
        &self,
        display_id: DisplayId,
    ) -> <P as ViewResources<'_>>::FrameUpdate {
        <P as ViewResources<'_>>::to_frame_update(&self.processing, &self.frame, display_id)
    }
}
