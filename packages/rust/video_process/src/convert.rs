use dcv_color_primitives as dcp;
use dcv_color_primitives::{
    convert_image,
    get_buffers_size,
    ColorSpace,
    ErrorKind,
    ImageFormat,
    PixelFormat,
};

pub fn convert_bgra_to_i420(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>, ErrorKind> {
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
    get_buffers_size(width, height, &dst_format, None, sizes)?;

    let mut i420_image = Vec::with_capacity(sizes[0]);

    convert_image(
        width,
        height,
        &src_format,
        None,
        &[data],
        &dst_format,
        None,
        &mut [&mut i420_image],
    )?;

    unsafe { i420_image.set_len(sizes[0]) };

    Ok(i420_image)
}
