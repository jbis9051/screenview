use crate::rtp::Vp9PacketWrapperBecauseTheRtpCrateIsIdiotic;
use cfg_if::cfg_if;
use num_cpus;
use std::{
    cmp::{max, min},
    os::raw::c_uint,
};
use vpx_sys::{
    vp8_dec_control_id::{VP9D_SET_LOOP_FILTER_OPT, VPXD_GET_LAST_QUANTIZER},
    vpx_codec_control_,
    vpx_codec_ctx_t,
    vpx_codec_cx_pkt_kind::VPX_CODEC_CX_FRAME_PKT,
    vpx_codec_dec_cfg_t,
    vpx_codec_dec_init_ver,
    vpx_codec_decode,
    vpx_codec_destroy,
    vpx_codec_enc_cfg_t,
    vpx_codec_enc_config_default,
    vpx_codec_enc_init_ver,
    vpx_codec_encode,
    vpx_codec_err_t,
    vpx_codec_flags_t,
    vpx_codec_get_cx_data,
    vpx_codec_get_frame,
    vpx_codec_iter_t,
    vpx_codec_vp9_cx,
    vpx_codec_vp9_dx,
    vpx_image_t,
    vpx_img_alloc,
    vpx_img_fmt,
    vpx_img_fmt::VPX_IMG_FMT_I420,
    vpx_img_free,
    vpx_img_wrap,
    VPX_DECODER_ABI_VERSION,
    VPX_DL_REALTIME,
    VPX_IMG_FMT_HIGHBITDEPTH,
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
                VPX_IMG_FMT_I420,
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

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn encode(&mut self, i420_frame: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        unsafe {
            vpx_img_wrap(
                self.raw,
                VPX_IMG_FMT_I420,
                self.width,
                self.height,
                1,
                i420_frame.as_ptr() as _,
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
    #[error("unsupported image format")]
    DecoderUnsupportedFormat,
}

impl From<vpx_codec_err_t> for Error {
    fn from(error: vpx_codec_err_t) -> Self {
        Self::VpxCodec(error)
    }
}


fn vpx_img_plane_width(img: &vpx_image_t, plane: usize) -> u32 {
    if plane > 0 && img.x_chroma_shift > 0 {
        (img.d_w + 1) >> img.x_chroma_shift
    } else {
        img.d_w
    }
}

fn vpx_img_plane_height(img: &vpx_image_t, plane: usize) -> u32 {
    if plane > 0 && img.y_chroma_shift > 0 {
        (img.d_h + 1) >> img.y_chroma_shift
    } else {
        img.d_h
    }
}

struct Vp9Decoder {
    width: u16,
    height: u16,

    buffer: Vec<u8>,
    decoder: vpx_codec_ctx_t,
}

impl Vp9Decoder {
    pub fn new(width: u16, height: u16) -> Result<Self, Error> {
        let number_of_cores = num_cpus::get();
        let mut cfg: vpx_codec_dec_cfg_t = unsafe { std::mem::zeroed() };
        // We want to use multithreading when decoding high resolution videos. But
        // not too many in order to avoid overhead when many stream are decoded
        // concurrently.
        // Set 2 thread as target for 1280x720 pixel count, and then scale up
        // linearly from there - but cap at physical core count.
        // For common resolutions this results in:
        // 1 for 360p
        // 2 for 720p
        // 4 for 1080p
        // 8 for 1440p
        // 18 for 4K
        let num_threads = max(1, 2 * ((width * height) as u32) / (1280u32 * 720u32));
        cfg.threads = min(number_of_cores as c_uint, num_threads as c_uint);

        let flags: vpx_codec_flags_t = 0;

        let mut decoder: vpx_codec_ctx_t = unsafe { std::mem::zeroed() };

        vp9_call_unsafe!(vpx_codec_dec_init_ver(
            &mut decoder,
            vpx_codec_vp9_dx(),
            &cfg,
            flags,
            VPX_DECODER_ABI_VERSION as _,
        ));

        vp9_call_unsafe!(vpx_codec_control_(
            &mut decoder,
            VP9D_SET_LOOP_FILTER_OPT as _,
            1
        ));

        let buffer = vec![0u8; (width * height * 4) as usize];

        Ok(Self {
            width,
            height,
            buffer,
            decoder,
        })
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        let mut buffer = data.as_ptr();
        if data.is_empty() {
            buffer = std::ptr::null(); // Triggers full frame concealment.
        }

        // During decode libvpx may get and release buffers from |frame_buffer_pool_|.
        // In practice libvpx keeps a few (~3-4) buffers alive at a time.
        vp9_call_unsafe!(vpx_codec_decode(
            &mut self.decoder,
            buffer,
            data.len() as _,
            std::ptr::null_mut(),
            VPX_DL_REALTIME as i64
        ));

        let mut vec = Vec::new();
        let mut iter: vpx_codec_iter_t = std::ptr::null();
        loop {
            let img = unsafe { vpx_codec_get_frame(&mut self.decoder, &mut iter) };
            if img.is_null() {
                break;
            }
            let img = unsafe { &*img };

            if img.fmt != VPX_IMG_FMT_I420 {
                return Err(Error::DecoderUnsupportedFormat);
            }

            let mut out = Vec::with_capacity((self.width * self.height * 3) as usize);
            let mut ptr = out.as_mut_ptr();

            for plane in 0 .. 3 {
                let mut buf = img.planes[plane];
                let stride = img.stride[plane];
                let w = vpx_img_plane_width(img, plane)
                    * (if img.fmt as u32 & VPX_IMG_FMT_HIGHBITDEPTH == VPX_IMG_FMT_HIGHBITDEPTH {
                        2
                    } else {
                        1
                    });
                let h = vpx_img_plane_height(img, plane);
                let mut y = 0;
                while y < h {
                    unsafe { std::ptr::copy_nonoverlapping(buf, ptr, w as usize) };
                    buf = unsafe { buf.add(stride as usize) };
                    ptr = unsafe { ptr.add(w as usize) };
                    y += 1;
                }
            }

            unsafe { out.set_len((self.width * self.height * 3) as usize) };
            vec.push(out);
        }

        Ok(vec)
    }
}

pub struct Vp9DecoderWrapper {
    decoder: Option<Vp9Decoder>,
}

impl Default for Vp9DecoderWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl Vp9DecoderWrapper {
    pub fn new() -> Self {
        Self { decoder: None }
    }

    fn init_decoder(&mut self, width: u16, height: u16) -> Result<(), Error> {
        let mut decoder = Vp9Decoder::new(width, height)?;
        self.decoder = Some(decoder);
        Ok(())
    }

    pub fn decode(
        &mut self,
        data: Vp9PacketWrapperBecauseTheRtpCrateIsIdiotic,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let decoder = match &mut self.decoder {
            None => {
                if !data.1.y {
                    // We need to initialize the decoder with the correct width and height
                    // y indicates resolution data is present
                    // so we must skip this packet if the decoder hasn't been initialized yet and we don't have width and height data
                    return Ok(Vec::new());
                }
                self.init_decoder(data.1.width[0] as _, data.1.height[0] as _)?;
                self.decoder.as_mut().unwrap()
            }
            Some(d) =>
                if !data.1.y {
                    d
                } else if data.1.width[0] != d.width() || data.1.height[0] != d.height() {
                    // Resolution has changed, tear down and re-init a new decoder in
                    // order to get correct sizing.
                    self.init_decoder(data.1.width[0] as _, data.1.height[0] as _)?;
                    self.decoder.as_mut().unwrap()
                } else {
                    d
                },
        };

        decoder.decode(&data.0)
    }
}
