use bytes::Bytes;
use rtp::{
    codecs::vp9::{Vp9Packet, Vp9Payloader},
    packet::Packet,
    packetizer::{new_packetizer, Depacketizer, Packetizer},
    sequence::WrappingSequencer,
};

const VP9_PAYLOAD_TYPE: u8 = 69; // TODO no idea what this is supposed to be

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


pub fn decode_vp9(rtp: Vec<u8>) -> Result<Vp9Packet, rtp::Error> {
    let mut packet = Vp9Packet::default();
    packet.depacketize(&Bytes::from(rtp))?;
    Ok(packet)
}
