use bytes::Bytes;
use rtp::{
    codecs::vp9::{Vp9Packet, Vp9Payloader},
    packet::Packet,
    packetizer::{new_packetizer, Depacketizer, Packetizer},
    sequence::WrappingSequencer,
};
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

pub struct RtpDecoder {}

impl Default for RtpDecoder {
    fn default() -> Self {
        Self::new()
    }
}


pub type Vp9PacketWrapperBecauseTheRtpCrateIsIdiotic = (Bytes, Vp9Packet);

impl RtpDecoder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn decode_to_vp9(
        &mut self,
        rtp: Vec<u8>,
    ) -> Result<Option<Vp9PacketWrapperBecauseTheRtpCrateIsIdiotic>, DecoderError> {
        let pkt = Packet::unmarshal(&mut Bytes::from(rtp))?;
        let mut vp9packet = Vp9Packet::default();
        let bytes = vp9packet.depacketize(&pkt.payload)?;
        Ok(Some((bytes, vp9packet)))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecoderError {
    #[error("{0}")]
    PacketUnmarshal(#[from] webrtc_util::Error),
    #[error("{0}")]
    Vp9Depacketize(#[from] rtp::Error),
}
