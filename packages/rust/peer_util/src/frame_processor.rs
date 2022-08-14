use crate::encoder::Encoder;
use capture::{FrameProcessResult, ProcessFrame, ViewResources};
use common::messages::rvd::DisplayId;
use native::api::BGRAFrame;
use rtp::packet::Packet;
use std::vec::Drain;
use video_process::{
    convert::bgra_to_i420,
    rtp::RtpEncoder,
    vp9::{self, VP9Encoder, Vp9Decoder},
};

pub struct FrameProcessor {
    encoder: Encoder,
}

impl ProcessFrame for FrameProcessor {
    type InitArgs = usize;
    type Resources = Vec<Packet>;

    // MTU

    fn new(args: Self::InitArgs) -> Self {
        Self {
            encoder: Encoder::new(args),
        }
    }

    fn process(
        &mut self,
        frame: &mut BGRAFrame,
        resources: &mut Self::Resources,
    ) -> FrameProcessResult {
        match self.encoder.process(frame) {
            Ok(rtp_packets) => resources.extend(rtp_packets),
            Err(_) => return FrameProcessResult::Failure,
        };

        FrameProcessResult::Success
    }
}

impl<'a> ViewResources<'a> for FrameProcessor {
    type FrameUpdate = Drain<'a, Packet>;
    type Resources = <Self as ProcessFrame>::Resources;

    #[inline]
    fn frame_update(
        resources: &'a mut Self::Resources,
        _frame: &'a BGRAFrame,
        _display_id: DisplayId,
    ) -> Self::FrameUpdate {
        resources.drain(..)
    }
}
