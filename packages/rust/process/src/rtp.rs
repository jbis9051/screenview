use crate::{vp9, vp9::VP9Encoder};
use bytes::Bytes;
use dcv_color_primitives as dcp;
use dcv_color_primitives::{convert_image, get_buffers_size, ColorSpace, ImageFormat, PixelFormat};
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

    pub fn process_bgra(
        &mut self,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<Vec<Packet>, Error> {
        dcp::initialize();


        let src_format = ImageFormat {
            pixel_format: PixelFormat::Bgra,
            color_space: ColorSpace::Rgb,
            num_planes: 1,
        };

        let dst_format = ImageFormat {
            pixel_format: PixelFormat::I420,
            color_space: ColorSpace::Bt601,
            num_planes: 1,
        };

        let sizes: &mut [usize] = &mut [0usize; 1];
        get_buffers_size(width, height, &dst_format, None, sizes).map_err(Error::Converter)?;

        let mut i420_image = Vec::with_capacity(sizes[0]);

        convert_image(
            width,
            height,
            &src_format,
            None,
            &[&data],
            &dst_format,
            None,
            &mut [&mut i420_image],
        )
        .map_err(Error::Converter)?;

        unsafe { i420_image.set_len(sizes[0]) }


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
    #[error("DCP converter error: {0}")]
    Converter(dcp::ErrorKind),
}
