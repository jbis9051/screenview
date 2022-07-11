use cfg_if::cfg_if;
use vpx_sys::{
    vpx_codec_control_,
    vpx_codec_ctx_t,
    vpx_codec_cx_pkt_kind::VPX_CODEC_CX_FRAME_PKT,
    vpx_codec_destroy,
    vpx_codec_enc_cfg_t,
    vpx_codec_enc_config_default,
    vpx_codec_enc_init_ver,
    vpx_codec_encode,
    vpx_codec_err_t,
    vpx_codec_get_cx_data,
    vpx_codec_vp9_cx,
    vpx_image_t,
    vpx_img_alloc,
    vpx_img_fmt,
    vpx_img_free,
    vpx_img_wrap,
    VPX_DL_REALTIME,
};

// For the next soul that is looking for documentation, see: https://developer.liveswitch.io/reference/cocoa/api/group__encoder.html, https://docs.freeswitch.org/switch__image_8h.html

pub struct VP9Encoder {
    encoder: vpx_codec_ctx_t,
    config: vpx_codec_enc_cfg_t,

    width: u32,
    height: u32,

    raw: *mut vpx_image_t,
    pts: i64,
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

        let raw = unsafe {
            vpx_img_alloc(
                std::ptr::null::<vpx_image_t>() as _,
                vpx_img_fmt::VPX_IMG_FMT_I420,
                width,
                height,
                1,
            )
        };

        if raw.is_null() {
            return Err(Error::VpxAlloc);
        }


        Ok(VP9Encoder {
            encoder,
            config,
            width,
            height,
            raw,
            pts: 0,
        })
    }

    pub fn encode(&mut self, frame: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        unsafe {
            vpx_img_wrap(
                self.raw,
                vpx_img_fmt::VPX_IMG_FMT_I420,
                self.width,
                self.height,
                1,
                frame.as_ptr() as _,
            )
        };
        vp9_call_unsafe!(vpx_codec_encode(
            &mut self.encoder,
            self.raw,
            self.pts,
            1,
            0,
            VPX_DL_REALTIME.into(),
        ));


        let mut datas = Vec::new();

        let mut iter = std::ptr::null();
        loop {
            let pkt = unsafe { vpx_codec_get_cx_data(&mut self.encoder, &mut iter) };
            if pkt.is_null() {
                break;
            }
            let pkt = unsafe { &*pkt };
            if pkt.kind != VPX_CODEC_CX_FRAME_PKT {
                break;
            }
            let mut data = Vec::<u8>::with_capacity(unsafe { pkt.data.frame.sz } as usize);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    pkt.data.frame.buf as _,
                    data.as_mut_ptr(),
                    pkt.data.frame.sz as usize,
                );
            }
            unsafe { data.set_len(pkt.data.frame.sz as usize) };
            datas.push(data);
        }
        Ok(datas)
    }
}

impl Drop for VP9Encoder {
    fn drop(&mut self) {
        unsafe {
            vpx_codec_destroy(&mut self.encoder);
        }
        if !self.raw.is_null() {
            unsafe { vpx_img_free(self.raw) };
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
    #[error("vpx_img_alloc failed")]
    VpxAlloc,
}

impl From<vpx_codec_err_t> for Error {
    fn from(error: vpx_codec_err_t) -> Self {
        Self::VpxCodec(error)
    }
}
