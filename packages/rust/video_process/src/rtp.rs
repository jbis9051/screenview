use crate::vp9::{Vp9Frame, Vp9FrameMeta};
use bytes::Bytes;
use rtp::{
    codecs::vp9::{Vp9Packet, Vp9Payloader},
    packet::Packet,
    packetizer::{new_packetizer, Depacketizer, Packetizer},
    sequence::WrappingSequencer,
};
use webrtc_media::{io::sample_builder::SampleBuilder, Sample};
use webrtc_util::Unmarshal;

const VP9_PAYLOAD_TYPE: u8 = 98;

pub struct RtpEncoder {
    packetizer: Box<dyn Packetizer>,
}

impl RtpEncoder {
    pub fn new(mtu: usize, ssrc: u32) -> Self {
        let payloader = Vp9Payloader::new();
        let sequenceizer = WrappingSequencer::new(0);
        let packetizer =
            new_packetizer(mtu, VP9_PAYLOAD_TYPE, ssrc, payloader, sequenceizer, 90000);
        Self {
            packetizer: Box::new(packetizer),
        }
    }

    pub fn process_vp9(&mut self, vp9_frame: Vec<u8>) -> Result<Vec<Packet>, rtp::Error> {
        let bytes = Bytes::from(vp9_frame);
        self.packetizer.packetize(&bytes, 1)
    }
}

pub struct RtpDecoder {
    builder: SampleBuilder<Vp9Packet>,
}

impl Default for RtpDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl RtpDecoder {
    pub fn new() -> Self {
        Self {
            builder: SampleBuilder::new(100, Vp9Packet::default(), 1), // TODO
        }
    }

    pub fn decode_to_vp9(&mut self, rtp: Vec<u8>) -> Vec<Sample> {
        // TODO accept Packet
        let mut out = Vec::new();
        let pkt = match Packet::unmarshal(&mut Bytes::from(rtp)).ok() {
            None => return out,
            Some(pkt) => pkt,
        };
        self.builder.push(pkt);
        while let Some(sample) = self.builder.pop() {
            out.push(sample);
        }
        out
    }
}
