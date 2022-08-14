use capture::FrameProcessResult;
use native::api::BGRAFrame;
use rtp::packet::Packet;
use std::env::args;
use video_process::{
    convert::bgra_to_i420,
    rtp::RtpEncoder,
    vp9,
    vp9::{VP9Encoder, Vp9Frame},
};

pub struct Encoder {
    vp9_encoder: Option<VP9Encoder>,
    rtp_encoder: RtpEncoder,
}


impl Encoder {
    pub fn new(mtu: usize) -> Self {
        Self {
            vp9_encoder: None,
            rtp_encoder: RtpEncoder::new(mtu, 0),
        }
    }

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

    pub fn process(&mut self, frame: &mut BGRAFrame) -> Result<Vec<Packet>, ()> {
        // TODO: maybe log information about the error
        if self.lazy_init_vp9(frame).is_err() {
            return Err(());
        }

        let i420_frame = match bgra_to_i420(frame.width, frame.height, &mut frame.data) {
            Ok(data) => data,
            Err(_) => return Err(()),
        };

        // unwrap is fine because we ensure the encoder is present with the check above, this
        // branch should be optimized out by the compiler
        let vp9_encoder = self.vp9_encoder.as_mut().unwrap();

        let vp9_frames = match vp9_encoder.encode(&i420_frame) {
            Ok(packets) => packets,
            // TODO: log more detailed information about the error
            Err(_) => return Err(()),
        };

        // We don't need to clear the resources because that's done when they're viewed

        let mut rtp_packets = Vec::new();

        for vp9_frame in vp9_frames {
            match self.rtp_encoder.process_vp9(vp9_frame.data) {
                Ok(packets) => rtp_packets.extend(packets),
                // TODO: log more detailed information about the error
                Err(_) => continue,
            };
        }

        return Ok(rtp_packets);
    }
}
