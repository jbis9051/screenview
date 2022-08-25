use dcv_color_primitives::ErrorKind;
use image::{DynamicImage, ImageBuffer, ImageError, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;
use video_process::{
    convert::{bgra_to_rgb, i420_to_bgra},
    rtp::RtpDecoder,
    vp9,
    vp9::Vp9Decoder,
};

pub struct Decoder {
    rtp: RtpDecoder,
    vp9: Vp9Decoder,
}

pub struct Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Decoder {
    pub fn new() -> Result<Self, vp9::Error> {
        Ok(Self {
            rtp: RtpDecoder::new(),
            vp9: Vp9Decoder::new()?,
        })
    }

    pub fn process(&mut self, rtp_data: Vec<u8>) -> Result<Vec<Frame>, Error> {
        let mut out = Vec::new();

        let samples = self.rtp.decode_to_vp9(rtp_data);

        for sample in samples {
            let frames = self.vp9.decode(&sample.data)?;

            for frame in frames {
                out.push(Frame {
                    width: frame.meta.width,
                    height: frame.meta.height,
                    data: frame.data,
                });
            }
        }

        Ok(out)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Vp9Decode error: {0}")]
    Vp9Decode(#[from] vp9::Error),
    #[error("Decoder error: {0}")]
    Decoder(#[from] ErrorKind),
    #[error("Image error: {0}")]
    Image(#[from] ImageError),
}
