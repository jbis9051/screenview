use cfg_if::cfg_if;
use native::api::Frame;
use std::mem;
use vpx_sys::*;

pub struct VpxImageWrapper(vpx_image_t);

impl Drop for VpxImageWrapper {
    fn drop(&mut self) {
        unsafe {
            vpx_img_free(&mut self.0);
        }
    }
}

pub struct VP9Encoder {
    encoder: vpx_codec_ctx_t,
    config: vpx_codec_enc_cfg_t,

    width: u32,
    height: u32,
}

macro_rules! vp9_call_unsafe {
    ($expr: expr) => {{
        let res = unsafe { $expr };
        if res != vpx_codec_err_t::VPX_CODEC_OK {
            return Err(res.into());
        }
    }};
}

impl VP9Encoder {
    pub fn new(width: u32, height: u32) -> Result<VP9Encoder, Error> {
        let mut encoder: vpx_codec_ctx_t = unsafe { std::mem::zeroed() };
        let mut config: vpx_codec_enc_cfg_t = unsafe { std::mem::zeroed() };


        vp9_call_unsafe!(vpx_codec_enc_config_default(
            vpx_codec_vp9_cx(),
            &mut config,
            0
        ));
        vp9_call_unsafe!(vpx_codec_enc_init_ver(
            &mut encoder,
            vpx_codec_vp9_cx(),
            &config,
            0,
            vpx_sys::vpx_bit_depth::VPX_BITS_8 as i32,
        ));
        vp9_call_unsafe!(vpx_codec_control_(
            &mut encoder,
            vpx_sys::vp8e_enc_control_id::VP8E_SET_CPUUSED as _,
            get_cpu_speed(width, height),
        ));
        vp9_call_unsafe!(vpx_codec_control_(
            &mut encoder,
            vpx_sys::vp8e_enc_control_id::VP9E_SET_ROW_MT as _,
            1
        ));


        Ok(VP9Encoder {
            encoder,
            config,
            width,
            height,
        })
    }

    pub fn encode(&self, frame: &mut [u8]) -> VpxImageWrapper {
        let mut image = unsafe { mem::zeroed() };

        let res = unsafe {
            vpx_img_wrap(
                &mut image,
                vpx_img_fmt::VPX_IMG_FMT_I420,
                self.width as _,
                self.height as _,
                1,
                frame.as_mut_ptr(),
            )
        };

        if unsafe { res.as_ref() }.is_none() {
            panic!("Failed to wrap image");
        }

        VpxImageWrapper(image)
    }
}

impl Drop for VP9Encoder {
    fn drop(&mut self) {
        unsafe {
            vpx_codec_destroy(&mut self.encoder);
        }
    }
}

// Only positive speeds, range for real-time coding currently is: 5 - 8.
// Lower means slower/better quality, higher means fastest/lower quality.
fn get_cpu_speed(width: u32, height: u32) -> i32 {
    cfg_if! {
       if #[cfg(target_arch = "arm64")]{
            return 8;
        } else {
            // For smaller resolutions, use lower speed setting (get some coding gain at
             // the cost of increased encoding complexity).
            if width * height <= 352 * 288 {
                5
            } else {
                7
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("VPX Codec error: {0:?}")]
    VpxCodec(vpx_codec_err_t),
}

impl From<vpx_codec_err_t> for Error {
    fn from(error: vpx_codec_err_t) -> Self {
        Self::VpxCodec(error)
    }
}
