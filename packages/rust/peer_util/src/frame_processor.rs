use capture::{FrameProcessResult, ProcessFrame, ViewResources};
use common::messages::rvd::DisplayId;
use native::api::BGRAFrame;
use std::iter::FusedIterator;

#[derive(Default)]
pub struct FrameProcessor {}

impl ProcessFrame for FrameProcessor {
    type Resources = ();

    fn process(
        &mut self,
        _frame: &mut BGRAFrame,
        _resources: &mut Self::Resources,
    ) -> FrameProcessResult {
        // TODO: this will contain the logic that implements a protocol like VP9
        FrameProcessResult::Success
    }
}

impl<'a> ViewResources<'a> for FrameProcessor {
    type FrameUpdate = FrameUpdate<'a>;
    type Resources = <Self as ProcessFrame>::Resources;

    fn frame_update(
        _resources: &'a Self::Resources,
        frame: &'a BGRAFrame,
        display_id: DisplayId,
    ) -> Self::FrameUpdate {
        FrameUpdate::new(frame, display_id)
    }
}

pub struct FrameUpdate<'a> {
    frame: &'a BGRAFrame,
    pub(crate) display_id: DisplayId,
    message_fragment_returned: bool,
}

impl<'a> FrameUpdate<'a> {
    fn new(frame: &'a BGRAFrame, display_id: DisplayId) -> Self {
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
            data: self.frame.data.clone(),
        })
    }
}

impl<'a> FusedIterator for FrameUpdate<'a> {}

pub struct FrameDataMessageFragment {
    pub(crate) cell_number: u16,
    pub(crate) data: Vec<u8>,
}
