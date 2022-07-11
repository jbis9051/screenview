use common::messages::rvd::DisplayId;
use native::api::BGRAFrame;

pub trait ProcessFrame: Default + 'static {
    type Resources: Send + Default;

    // TODO: consider giving more detailed error information
    fn process(
        &mut self,
        frame: &mut BGRAFrame,
        resources: &mut Self::Resources,
    ) -> FrameProcessResult;
}

pub trait ViewResources<'a> {
    type FrameUpdate;
    type Resources;

    fn frame_update(
        resources: &'a Self::Resources,
        frame: &'a BGRAFrame,
        display_id: DisplayId,
    ) -> Self::FrameUpdate;
}

#[derive(Clone, Copy)]
pub enum FrameProcessResult {
    Success,
    Failure,
}
