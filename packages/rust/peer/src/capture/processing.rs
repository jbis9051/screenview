use std::iter::FusedIterator;

use common::messages::rvd::DisplayId;
use native::api::Frame;

pub trait ProcessFrame: Default + 'static {
    type Resources: Send + Default;

    // TODO: consider giving more detailed error information
    fn process(&mut self, frame: &mut Frame, resources: &mut Self::Resources)
        -> FrameProcessResult;
}

pub trait ViewResources<'a> {
    type Resources;
    type FrameUpdate;

    fn to_frame_update(
        resources: &'a Self::Resources,
        frame: &'a Frame,
        display_id: DisplayId,
    ) -> Self::FrameUpdate;
}

#[derive(Clone, Copy)]
pub enum FrameProcessResult {
    Success,
    Failure,
}

#[derive(Default)]
pub struct DefaultFrameProcessor {}

impl ProcessFrame for DefaultFrameProcessor {
    type Resources = ();

    fn process(
        &mut self,
        _frame: &mut Frame,
        _resources: &mut Self::Resources,
    ) -> FrameProcessResult {
        // TODO: this will contain the logic that implements a protocol like VP9
        FrameProcessResult::Success
    }
}

impl<'a> ViewResources<'a> for DefaultFrameProcessor {
    type FrameUpdate = FrameUpdate<'a>;
    type Resources = <Self as ProcessFrame>::Resources;

    fn to_frame_update(
        _resources: &'a Self::Resources,
        frame: &'a Frame,
        display_id: DisplayId,
    ) -> Self::FrameUpdate {
        FrameUpdate::new(frame, display_id)
    }
}

pub struct FrameUpdate<'a> {
    frame: &'a Frame,
    pub(crate) display_id: DisplayId,
    message_fragment_returned: bool,
}

impl<'a> FrameUpdate<'a> {
    fn new(frame: &'a Frame, display_id: DisplayId) -> Self {
        Self {
            frame,
            display_id,
            message_fragment_returned: false,
        }
    }
}

impl<'a> Iterator for FrameUpdate<'a> {
    type Item = FrameDataMessageFragment;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: split into chunks

        if self.message_fragment_returned {
            return None;
        }

        self.message_fragment_returned = true;
        Some(FrameDataMessageFragment {
            cell_number: 0,
            data: self.frame.as_raw().clone(),
        })
    }
}

impl<'a> FusedIterator for FrameUpdate<'a> {}

pub struct FrameDataMessageFragment {
    pub(crate) cell_number: u16,
    pub(crate) data: Vec<u8>,
}
