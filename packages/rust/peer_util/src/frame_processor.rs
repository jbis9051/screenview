use capture::{FrameProcessResult, ProcessFrame, ViewResources};
use common::messages::rvd::DisplayId;
use dcv_color_primitives::ErrorKind;
use native::api::BGRAFrame;
use rtp::packet::Packet;
use std::vec::Drain;
use video_process::{
    convert::convert_bgra_to_i420,
    rtp::RtpEncoder,
    vp9::{self, VP9Encoder},
};

pub struct FrameProcessor {
    vp9_encoder: Option<VP9Encoder>,
    rtp_encoder: RtpEncoder,
}

impl Default for FrameProcessor {
    fn default() -> Self {
        // TODO: get real values for the MTU and SSRC
        Self {
            vp9_encoder: None,
            rtp_encoder: RtpEncoder::new(io::DEFAULT_UNRELIABLE_MESSAGE_SIZE, 0),
        }
    }
}

impl FrameProcessor {
    #[inline]
    fn lazy_init_vp9(&mut self, incoming: &BGRAFrame) -> Result<(), vp9::Error> {
        let stale = self
            .vp9_encoder
            .as_ref()
            .map(|encoder| encoder.dimensions() != (incoming.width, incoming.height))
            .unwrap_or(true);

        // TODO: we remake the encoder if the incoming frame size changes, is this the right
        // thing to do?
        if stale {
            self.vp9_encoder = Some(VP9Encoder::new(incoming.width, incoming.height)?);
        }

        Ok(())
    }
}

impl ProcessFrame for FrameProcessor {
    type Resources = Vec<Packet>;

    fn process(
        &mut self,
        frame: &mut BGRAFrame,
        resources: &mut Self::Resources,
    ) -> FrameProcessResult {
        // TODO: maybe log information about the error
        if self.lazy_init_vp9(frame).is_err() {
            return FrameProcessResult::Failure;
        }

        let i420_frame = match convert_bgra_to_i420(frame.width, frame.height, &mut frame.data) {
            Ok(data) => data,
            Err(_) => return FrameProcessResult::Failure,
        };

        // unwrap is fine because we ensure the encoder is present with the check above, this
        // branch should be optimized out by the compiler
        let vp9_encoder = self.vp9_encoder.as_mut().unwrap();

        let packets = match vp9_encoder.encode(&i420_frame) {
            Ok(packets) => packets,
            // TODO: log more detailed information about the error
            Err(_) => return FrameProcessResult::Failure,
        };

        // We don't need to clear the resources because that's done when they're viewed

        for packet in packets {
            let rtp_packets = match self.rtp_encoder.process_vp9(packet) {
                Ok(rtp_packets) => rtp_packets,
                // TODO: log more detailed information about the error
                Err(_) => return FrameProcessResult::Failure,
            };

            resources.extend(rtp_packets);
        }

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
