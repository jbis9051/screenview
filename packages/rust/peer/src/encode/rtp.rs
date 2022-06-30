use crate::encode::{vp9, vp9::VP9Encoder};
use bytes::Bytes;
use native::api::Frame;
use rtp::{
    codecs::vp9::Vp9Payloader,
    packet::Packet,
    packetizer::{new_packetizer, Packetizer},
    sequence::WrappingSequencer,
};

const VP9_PAYLOAD_TYPE: u8 = 69; // TODO no idea what this is supposed to be

pub struct RtpEncoder {
    encoder: VP9Encoder,
    packetizer: Box<dyn Packetizer>,
}

impl RtpEncoder {
    pub fn new(mtu: usize, ssrc: u32, width: u32, height: u32) -> Result<Self, Error> {
        let encoder = VP9Encoder::new(width, height).map_err(Error::Encoder)?;
        let payloader = Vp9Payloader::new();
        let sequenceizer = WrappingSequencer::new(0);
        let packetizer =
            new_packetizer(mtu, VP9_PAYLOAD_TYPE, ssrc, payloader, sequenceizer, 90000);
        Ok(Self {
            encoder,
            packetizer: Box::new(packetizer),
        })
    }

    pub fn process(&mut self, _frame: Frame) -> Result<Vec<Packet>, Error> {
        let i420_image = Vec::new(); // TODO convert frame to i420 type
        let datas = self.encoder.encode(&i420_image).map_err(Error::Encoder)?;
        let mut packets = Vec::new();
        for data in datas {
            let bytes = Bytes::from(data);

            packets.append(
                &mut self
                    .packetizer
                    .packetize(&bytes, 1)
                    .map_err(Error::Packetizer)?,
            );
        }

        Ok(packets)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("VP9 encoder error: {0}")]
    Encoder(vp9::Error),
    #[error("RTP packetizer error: {0}")]
    Packetizer(rtp::Error),
}
