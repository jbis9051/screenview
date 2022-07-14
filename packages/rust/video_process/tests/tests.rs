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
    vp9::{VP9Encoder, Vp9Decoder},
};

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
pub fn decode_test() {
    let img = include_bytes!("img.vp9");
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
pub fn e2e_encode() {
    let png =
        image::load_from_memory_with_format(include_bytes!("img.png"), image::ImageFormat::Png)
            .expect("unable to open image");
    let rgb = png.to_rgb8();
    let (width, height) = rgb.dimensions();
    let rgb_data = rgb.into_raw();
    let mut bgra = rgb_to_bgra(width, height, &rgb_data).expect("unable to convert image");
    let i420 = convert_bgra_to_i420(width, height, &mut bgra).expect("unable to convert image");
    let mut encoder = VP9Encoder::new(width, height).expect("could not construct encoder");

    let mut vp9 = encoder.encode(&i420).expect("could not encode frame");
    vp9.append(&mut encoder.encode(&[]).unwrap());
    let vp9_flat: Vec<u8> = vp9.into_iter().flatten().collect();
    let mut decoder =
        Vp9Decoder::new(width as _, height as _).expect("could not construct encoder");
    let mut vp9_dec = decoder.decode(&vp9_flat).expect("could not decode frame");
    vp9_dec.append(&mut decoder.decode(&[]).unwrap());
    let vp9_dec_flat: Vec<u8> = vp9_dec.into_iter().flatten().collect();
    let bgra_dec = i420_to_bgra(width, height, &vp9_dec_flat).expect("unable to convert image");
    let rgb_dec = bgra_to_rgb(width, height, &bgra_dec).expect("unable to convert image");
    RgbImage::from_vec(width, height, rgb_dec)
        .expect("unable to load image")
        .save("tests/out.png")
        .unwrap();
}
