#![allow(dead_code)]

use crate::rtp::Vp9PacketWrapperBecauseTheRtpCrateIsIdiotic;
use bytes::Bytes;
use cfg_if::cfg_if;
use num_cpus;
use std::{
    cmp::{max, min},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    os::raw::c_uint,
};
use vpx_sys::{
    vp8_dec_control_id::VP9D_SET_LOOP_FILTER_OPT,
    vpx_codec_control_,
    vpx_codec_ctx_t,
    vpx_codec_cx_pkt_kind::VPX_CODEC_CX_FRAME_PKT,
    vpx_codec_dec_cfg_t,
    vpx_codec_dec_init_ver,
    vpx_codec_decode,
    vpx_codec_destroy,
    vpx_codec_enc_cfg_t,
    vpx_codec_enc_config_default,
    vpx_codec_enc_config_set,
    vpx_codec_enc_init_ver,
    vpx_codec_encode,
    vpx_codec_err_t,
    vpx_codec_flags_t,
    vpx_codec_get_cx_data,
    vpx_codec_get_frame,
    vpx_codec_iter_t,
    vpx_codec_vp9_cx,
    vpx_codec_vp9_dx,
    vpx_enc_pass::VPX_RC_ONE_PASS,
    vpx_image_t,
    vpx_img_alloc,
    vpx_img_fmt,
    vpx_img_fmt::VPX_IMG_FMT_I420,
    vpx_img_free,
    vpx_img_wrap,
    vpx_rc_mode::VPX_CBR,
    VPX_DECODER_ABI_VERSION,
    VPX_DL_BEST_QUALITY,
    VPX_DL_REALTIME,
    VPX_EFLAG_FORCE_KF,
    VPX_ENCODER_ABI_VERSION,
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
        let mut config: MaybeUninit<vpx_codec_enc_cfg_t> = MaybeUninit::uninit();

        vp9_call_unsafe!(vpx_codec_enc_config_default(
            vpx_codec_vp9_cx(),
            config.as_mut_ptr(),
            0
        ));

        let mut config = unsafe { config.assume_init() };

        config.g_w = width;
        config.g_h = height; /*
                              config.rc_target_bitrate = 0; // in kbit/s
                             // config_->g_error_resilient = is_svc_ ? VPX_ERROR_RESILIENT_DEFAULT : 0;
                             // Setting the time base of the codec.
                             config.g_timebase.num = 1;
                             config.g_timebase.den = 90000;
                             */
        config.g_lag_in_frames = 0; // 0- no frame lagging
                                    /*
                                    // Rate control settings.
                                    // config_->rc_dropframe_thresh = inst->VP9().frameDroppingOn ? 30 : 0;
                                    config.rc_end_usage = VPX_CBR;
                                    config.g_pass = VPX_RC_ONE_PASS;
                                    config.rc_min_quantizer = 8;
                                    config.rc_max_quantizer = 52;
                                    config.rc_undershoot_pct = 50;
                                    config.rc_overshoot_pct = 50;
                                    config.rc_buf_initial_sz = 500;
                                    config.rc_buf_optimal_sz = 600;
                                    config.rc_buf_sz = 1000;
                                    // Set the maximum target size of any key-frame.
                                    // rc_max_intra_target_ = MaxIntraTarget(config_->rc_buf_optimal_sz);
                                    // Key-frame interval is enforced manually by this wrapper.
                                    // config.kf_mode = VPX_KF_DISABLED;
                                    // TODO(webm:1592): work-around for libvpx issue, as it can still
                                    // put some key-frames at will even in VPX_KF_DISABLED kf_mode.
                                    // config_->kf_max_dist = inst->VP9().keyFrameInterval;
                                    // config_->kf_min_dist = config_->kf_max_dist;*/
        // Determine number of threads based on the image size and #cores.
        config.g_threads = number_of_threads(width, height, num_cpus::get() as u32);

        vp9_call_unsafe!(vpx_codec_enc_init_ver(
            &mut encoder,
            vpx_codec_vp9_cx(),
            &config,
            0,
            VPX_ENCODER_ABI_VERSION as _,
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

        vp9_call_unsafe!(vpx_codec_enc_config_set(&mut encoder, &config));

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
        let img = {
            if i420_frame.is_empty() {
                std::ptr::null_mut()
            } else {
                unsafe {
                    vpx_img_wrap(
                        self.raw,
                        VPX_IMG_FMT_I420,
                        self.width,
                        self.height,
                        0,
                        i420_frame.as_ptr() as _,
                    )
                };
                self.raw
            }
        };
        self.encode_internal(img)
    }

    fn encode_internal(&mut self, raw: *mut vpx_image_t) -> Result<Vec<Vec<u8>>, Error> {
        let target_framerate_fps = 20;
        let duration = 90000 / target_framerate_fps;

        let flags = 0;

        vp9_call_unsafe!(vpx_codec_encode(
            &mut self.encoder,
            raw,
            self.pts,
            1,
            flags,
            VPX_DL_REALTIME as _,
        ));

        self.pts += duration;

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

fn number_of_threads(width: u32, height: u32, number_of_cores: u32) -> u32 {
    // Keep the number of encoder threads equal to the possible number of column
    // tiles, which is (1, 2, 4, 8). See comments below for VP9E_SET_TILE_COLUMNS.
    if width * height >= 1280 * 720 && number_of_cores > 4 {
        4
    } else if width * height >= 640 * 360 && number_of_cores > 2 {
        2
    } else {
        // Use 2 threads for low res on ARM.
        #[cfg(target_arch = "arm64")]
        {
            if width * height >= 320 * 180 && number_of_cores > 2 {
                return 2;
            }
        }
        // 1 thread less than VGA.
        1
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
    #[error("Resolution is None and it is not a keyframe :(")]
    KeyFrameNotSent,
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

struct DecoderWrapper(vpx_codec_ctx_t);

impl DecoderWrapper {
    fn new() -> Self {
        Self(unsafe { std::mem::zeroed() })
    }
}

impl Deref for DecoderWrapper {
    type Target = vpx_codec_ctx_t;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DecoderWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for DecoderWrapper {
    fn drop(&mut self) {
        unsafe {
            vpx_codec_destroy(&mut self.0);
        }
    }
}

pub struct Vp9Decoder {
    // None indicates the decoder is semi-uninitialized.
    // This is the case when the decoder is first created.
    // You can still call decode() but the first frame must be a keyframe
    // which will allow us to set the resolution. Once the resolution is set, this will be Some(width, height)
    // Further calls to decode will check that the resolution is the same as the previous call
    // If the resolution is different the decoder will be reinitlized.
    resolution: Option<(u16, u16)>,
    decoder: DecoderWrapper,
    number_of_cores: usize,
}

impl Vp9Decoder {
    pub fn new() -> Result<Self, Error> {
        let number_of_cores = num_cpus::get();
        let decoder = Self::init_decoder(number_of_cores, None)?;

        Ok(Self {
            resolution: None,
            decoder,
            number_of_cores,
        })
    }

    fn init_decoder(
        number_of_cores: usize,
        resolution: Option<(u16, u16)>,
    ) -> Result<DecoderWrapper, Error> {
        let mut cfg: vpx_codec_dec_cfg_t = unsafe { std::mem::zeroed() };
        if let Some(resolution) = resolution {
            let (width, height) = resolution;
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
            let num_threads = max(1, 2 * ((width as u32 * height as u32) / (1280u32 * 720u32)));
            cfg.threads = min(number_of_cores as c_uint, num_threads as c_uint);
        } else {
            // No config provided - don't know resolution to decode yet.
            // Set thread count to one in the meantime.
            cfg.threads = 1;
        }
        let flags: vpx_codec_flags_t = 0;

        let mut decoder = DecoderWrapper::new();

        vp9_call_unsafe!(vpx_codec_dec_init_ver(
            &mut *decoder,
            vpx_codec_vp9_dx(),
            &cfg,
            flags,
            VPX_DECODER_ABI_VERSION as _,
        ));

        vp9_call_unsafe!(vpx_codec_control_(
            &mut *decoder,
            VP9D_SET_LOOP_FILTER_OPT as _,
            1
        ));

        Ok(decoder)
    }

    fn decode_inner(
        &mut self,
        data: &Bytes,
        width: u16,
        height: u16,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let mut buffer = data.as_ptr();
        if data.is_empty() {
            buffer = std::ptr::null(); // Triggers full frame concealment.
        }

        // During decode libvpx may get and release buffers from |frame_buffer_pool_|.
        // In practice libvpx keeps a few (~3-4) buffers alive at a time.
        vp9_call_unsafe!(vpx_codec_decode(
            &mut *self.decoder,
            buffer,
            data.len() as _,
            std::ptr::null_mut(),
            VPX_DL_REALTIME as i64
        ));

        let mut vec = Vec::new();
        let mut iter: vpx_codec_iter_t = std::ptr::null();
        loop {
            let img = unsafe { vpx_codec_get_frame(&mut *self.decoder, &mut iter) };
            if img.is_null() {
                break;
            }
            let img = unsafe { &*img };

            if img.fmt != VPX_IMG_FMT_I420 {
                return Err(Error::DecoderUnsupportedFormat);
            }

            let mut out = Vec::with_capacity((width as usize * height as usize * 3) as usize);
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

            unsafe { out.set_len((width as usize * height as usize * 3) as usize) };
            vec.push(out);
        }

        Ok(vec)
    }

    pub fn decode(
        &mut self,
        pkt: Option<&Vp9PacketWrapperBecauseTheRtpCrateIsIdiotic>,
    ) -> Result<(Vec<Vec<u8>>, u16, u16), Error> {
        let pkt = match pkt {
            None =>
                return if let Some((width, height)) = self.resolution {
                    Ok((
                        self.decode_inner(&Bytes::new(), width, height)?,
                        width,
                        height,
                    ))
                } else {
                    Ok((Vec::new(), 0, 0))
                },
            Some(pkt) => pkt,
        };
        let header = &pkt.1;

        let (width, height) = {
            if header.p {
                // p indicates this is a key frame or kVideoFrameKey in libwebrtc
                match self.resolution {
                    Some((width, height))
                        if width == header.width[0] && height == header.height[0] =>
                        (width, height),
                    _ => {
                        // We don't have a resolution or the resolution has changed - reinitialize the decoder
                        let resolution = (header.width[0], header.height[0]);
                        self.resolution = Some(resolution);
                        self.decoder = Self::init_decoder(self.number_of_cores, self.resolution)?;
                        resolution
                    }
                }
            } else if let Some(resolution) = self.resolution {
                resolution
            } else {
                return Err(Error::KeyFrameNotSent);
            }
        };

        let data = &pkt.0;
        let vec = self.decode_inner(data, width, height)?;
        Ok((vec, width, height))
    }
}
