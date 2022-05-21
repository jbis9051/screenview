use std::iter::FusedIterator;

use common::messages::rvd::DisplayId;
use native::api::Frame;

use super::CaptureResources;

pub trait ProcessFrame: Default + Sized + 'static {
    type Resources: for<'a> ProcessorResources<'a>;

    fn process(&mut self, resources: &mut CaptureResources<Self>);
}

pub trait ProcessorResources<'a>: Send + Default {
    type FrameUpdate;

    fn to_frame_update(&'a self, frame: &'a Frame, display_id: DisplayId) -> Self::FrameUpdate;
}

#[derive(Default)]
pub struct DefaultFrameProcessor {}

impl ProcessFrame for DefaultFrameProcessor {
    type Resources = DefaultFrameProcessorResources;

    fn process(&mut self, _resources: &mut CaptureResources<Self>) {
        // TODO: this will contain the logic that implements a protocol like VP9
    }
}

#[derive(Default)]
pub struct DefaultFrameProcessorResources {}

impl<'a> ProcessorResources<'a> for DefaultFrameProcessorResources {
    type FrameUpdate = FrameUpdate<'a>;

    fn to_frame_update(&'a self, frame: &'a Frame, display_id: DisplayId) -> Self::FrameUpdate {
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
