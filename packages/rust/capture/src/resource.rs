use super::{processing::ProcessFrame, ViewResources};
use common::messages::rvd::DisplayId;
use native::api::BGRAFrame;

pub struct CaptureResources<P: ProcessFrame> {
    pub(super) frame: BGRAFrame,
    pub(super) processing: P::Resources,
}

impl<P: ProcessFrame> CaptureResources<P> {
    pub(super) fn new() -> Self {
        Self {
            frame: BGRAFrame {
                width: 0,
                height: 0,
                data: Vec::new(),
            },
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
        &mut self,
        display_id: DisplayId,
    ) -> <P as ViewResources<'_>>::FrameUpdate {
        <P as ViewResources<'_>>::frame_update(&mut self.processing, &self.frame, display_id)
    }
}
