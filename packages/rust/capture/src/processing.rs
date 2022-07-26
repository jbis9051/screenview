use common::messages::rvd::DisplayId;
use native::api::BGRAFrame;

pub trait ProcessFrame: 'static {
    type Resources: Send + Default;
    type InitArgs: Send;

    fn new(args: Self::InitArgs) -> Self;

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
        resources: &'a mut Self::Resources,
        frame: &'a BGRAFrame,
        display_id: DisplayId,
    ) -> Self::FrameUpdate;
}

#[derive(Clone, Copy)]
pub enum FrameProcessResult {
    Success,
    Failure,
}

impl FrameProcessResult {
    pub fn unwrap(self) {
        match self {
            FrameProcessResult::Success => (),
            FrameProcessResult::Failure => panic!("frame processing failed"),
        }
    }
}
