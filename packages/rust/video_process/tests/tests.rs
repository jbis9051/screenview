use dcv_color_primitives as dcp;
use dcv_color_primitives::{
    convert_image,
    get_buffers_size,
    ColorSpace,
    ErrorKind,
    ImageFormat,
    PixelFormat,
};
use image::{GenericImageView, RgbImage};
use video_process::{
    convert::convert_bgra_to_i420,
    rtp::{RtpDecoder, RtpEncoder},
    vp9::{VP9Encoder, Vp9Decoder},
};
use webrtc_util::Marshal;

pub fn rgb_to_bgra(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>, ErrorKind> {
    dcp::initialize();


    let src_format = ImageFormat {
        pixel_format: PixelFormat::Rgb,
        color_space: ColorSpace::Rgb,
        num_planes: 1,
    };

    let dst_format = ImageFormat {
        pixel_format: PixelFormat::Bgra,
        color_space: ColorSpace::Rgb,
        num_planes: 1,
    };

    let sizes: &mut [usize] = &mut [0usize; 1];
    get_buffers_size(width, height, &dst_format, None, sizes)?;

    let mut bgra = vec![0u8; sizes[0]];

    convert_image(
        width,
        height,
        &src_format,
        None,
        &[data],
        &dst_format,
        None,
        &mut [&mut bgra],
    )?;

    Ok(bgra)
}


pub fn bgra_to_rgb(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>, ErrorKind> {
    dcp::initialize();


    let src_format = ImageFormat {
        pixel_format: PixelFormat::Bgra,
        color_space: ColorSpace::Rgb,
        num_planes: 1,
    };

    let dst_format = ImageFormat {
        pixel_format: PixelFormat::Rgb,
        color_space: ColorSpace::Rgb,
        num_planes: 1,
    };

    let sizes: &mut [usize] = &mut [0usize; 1];
    get_buffers_size(width, height, &dst_format, None, sizes)?;

    let mut bgra = vec![0u8; sizes[0]];

    convert_image(
        width,
        height,
        &src_format,
        None,
        &[data],
        &dst_format,
        None,
        &mut [&mut bgra],
    )?;

    Ok(bgra)
}

pub fn i420_to_bgra(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>, ErrorKind> {
    dcp::initialize();

    let src_format = ImageFormat {
        pixel_format: PixelFormat::I420,
        color_space: ColorSpace::Bt601,
        num_planes: 3,
    };

    let dst_format = ImageFormat {
        pixel_format: PixelFormat::Bgra,
        color_space: ColorSpace::Rgb,
        num_planes: 1,
    };

    let src_sizes: &mut [usize] = &mut [0usize; 3];
    get_buffers_size(width, height, &src_format, None, src_sizes)?;
    let (y_data, uv_data) = data.split_at(src_sizes[0]);
    let (u_data, v_data) = uv_data.split_at(src_sizes[1]);

    let sizes: &mut [usize] = &mut [0usize; 1];
    get_buffers_size(width, height, &dst_format, None, sizes)?;
    let mut dst_data = vec![0u8; sizes[0]];

    convert_image(
        width,
        height,
        &src_format,
        None,
        &[y_data, u_data, v_data],
        &dst_format,
        None,
        &mut [&mut dst_data],
    )?;

    Ok(dst_data)
}


#[test]
pub fn convert_test() {
    let image =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image");
    let image = image.to_rgb8();
    let (width, height) = image.dimensions();
    let data = image.into_raw();
    let mut bgra = rgb_to_bgra(width, height, &data).expect("unable to convert image");
    let data = convert_bgra_to_i420(width, height, &mut bgra).expect("unable to convert image");
    assert!(!data.is_empty());
}


#[test]
pub fn encode_test() {
    let img = include_bytes!("img.i420");
    let (width, height) =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image")
            .dimensions();

    let mut encoder = VP9Encoder::new(width, height).expect("could not construct encoder");
    let mut bytes = encoder.encode(img).expect("could not encode frame");
    bytes.append(&mut encoder.encode(&[]).unwrap());
    assert!(!bytes.is_empty());
}

#[test]
pub fn rtp_encode() {
    let frame = include_bytes!("img.vp9");
    let mut rtp = RtpEncoder::new(10000, 1);
    let packets = rtp
        .process_vp9(frame.to_vec())
        .expect("could not encode frame");
    assert!(!packets.is_empty());
}

#[test]
pub fn rtp_decode() {
    let packet = include_bytes!("img.rtp");
    let mut rtp = RtpDecoder::new();
    let (data, _) = rtp
        .decode_to_vp9(packet.to_vec())
        .expect("could not decode frame")
        .expect("empty packet");
    assert!(!data.is_empty());
}

#[test]
pub fn decode_test() {
    let img = include_bytes!("img.rtp.out");
    let (width, height) =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image")
            .dimensions();

    let mut decoder =
        Vp9Decoder::new(width as _, height as _).expect("could not construct encoder");
    let mut bytes = decoder.decode(img).expect("could not decode frame");
    bytes.append(&mut decoder.decode(&[]).unwrap());
    assert!(!bytes.is_empty());
}

#[test]
pub fn finalize() {
    let img = &mut include_bytes!("img.i420.out").clone();
    let (width, height) =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image")
            .dimensions();

    let img = i420_to_bgra(width, height, img).expect("unable to convert image");
    let img = bgra_to_rgb(width, height, &img).expect("unable to convert image");
    RgbImage::from_vec(width, height, img).expect("unable to load image");
}

// Below test is used if you want to view the encoding result. It should not be run on CI.
pub fn e2e() -> (u32, u32, Vec<u8>) {
    // Start with a png
    let png =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image");

    // Get our RGB data
    let rgb = png.to_rgb8();
    let (width, height) = rgb.dimensions();
    let rgb_data = rgb.into_raw();

    // Convert to BGRA
    let mut bgra = rgb_to_bgra(width, height, &rgb_data).expect("unable to convert image");

    // Convert to I420
    let i420 = convert_bgra_to_i420(width, height, &mut bgra).expect("unable to convert image");

    // Encode to VP9
    let mut encoder = VP9Encoder::new(width, height).expect("could not construct encoder");
    let mut vp9 = encoder.encode(&i420).expect("could not encode frame");
    vp9.append(&mut encoder.encode(&[]).unwrap());

    let vp9_flat: Vec<u8> = vp9.into_iter().flatten().collect();
    // Packetize to RTP
    let mut rtp = RtpEncoder::new(100000, 1);
    let packets = rtp.process_vp9(vp9_flat).expect("could not encode frame");
    assert_eq!(packets.len(), 1);
    let packet = packets[0]
        .marshal()
        .expect("could not marshal packet")
        .to_vec();

    // Depacketize to VP9
    let mut rtp = RtpDecoder::new();
    let (vp9_out, _) = rtp
        .decode_to_vp9(packet)
        .expect("could not decode frame")
        .expect("empty packet");

    // Decode to VP9
    let mut decoder =
        Vp9Decoder::new(width as _, height as _).expect("could not construct encoder");
    let mut i420_out = decoder.decode(&vp9_out).expect("could not decode frame");
    i420_out.append(&mut decoder.decode(&[]).unwrap());
    let i420_out_flat: Vec<u8> = i420_out.into_iter().flatten().collect();
    let bgra_out = i420_to_bgra(width, height, &i420_out_flat).expect("unable to convert image");
    let rgb_out = bgra_to_rgb(width, height, &bgra_out).expect("unable to convert image");

    (width, height, rgb_out)
}

#[test]
pub fn e2e_test() {
    e2e();
}

pub fn e2e_demo() {
    let (width, height, rgb) = e2e();
    RgbImage::from_vec(width, height, rgb)
        .expect("unable to load image")
        .save("out.png")
        .unwrap();
}
