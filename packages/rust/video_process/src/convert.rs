use dcv_color_primitives as dcp;
use dcv_color_primitives::{convert_image, get_buffers_size, ColorSpace, ImageFormat, PixelFormat};
use fast_image_resize as fr;
use std::num::NonZeroU32;

pub use dcv_color_primitives::ErrorKind;

static SRC_FORMAT: ImageFormat = ImageFormat {
    pixel_format: PixelFormat::Bgra,
    color_space: ColorSpace::Rgb,
    num_planes: 1,
};

static DST_FORMAT: ImageFormat = ImageFormat {
    pixel_format: PixelFormat::I420,
    color_space: ColorSpace::Bt601,
    num_planes: 3,
};

fn convert_bgra_to_i420_efficient(
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<Vec<u8>, ErrorKind> {
    dcp::initialize();

    let sizes: &mut [usize] = &mut [0usize; 3];
    get_buffers_size(width, height, &DST_FORMAT, None, sizes)?;
    let mut dst_data = vec![0u8; sizes[0] + sizes[1] + sizes[2]];
    let (y_data, uv_data) = dst_data.split_at_mut(sizes[0]);
    let (u_data, v_data) = uv_data.split_at_mut(sizes[1]);

    convert_image(
        width,
        height,
        &SRC_FORMAT,
        None,
        &[data],
        &DST_FORMAT,
        None,
        &mut [y_data, u_data, v_data],
    )?;

    Ok(dst_data)
}

/// If width or height is odd, this function is much less efficient
pub fn convert_bgra_to_i420(
    width: u32,
    height: u32,
    data: &mut [u8],
) -> Result<Vec<u8>, ErrorKind> {
    if width & 1 == 0 && height & 1 == 0 {
        // if it's even just do the efficient one
        return convert_bgra_to_i420_efficient(width, height, data);
    }

    // Resize the image to be even width and height
    dcp::initialize();

    let new_width = width + (width & 1);
    let new_height = height + (height & 1);

    let src_image = fr::Image::from_slice_u8(
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
        data,
        fr::PixelType::U8x4,
    )
    .unwrap();

    let mut dst_image = fr::Image::new(
        NonZeroU32::new(new_width).unwrap(),
        NonZeroU32::new(new_height).unwrap(),
        src_image.pixel_type(),
    );
    let mut dst_view = dst_image.view_mut();


    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(fr::FilterType::Box));
    resizer.resize(&src_image.view(), &mut dst_view).unwrap();


    let new_data = dst_image.into_vec();

    // now we can convert to i420, but we will need to remove the extra pixels later so we can't use the efficient one even now

    let sizes: &mut [usize] = &mut [0usize; 3];
    get_buffers_size(new_width, new_height, &DST_FORMAT, None, sizes)?;

    let mut y_data = vec![0u8; sizes[0]]; // 4
    let mut u_data = vec![0u8; sizes[1]]; // 2
    let mut v_data = vec![0u8; sizes[2]]; // 2

    convert_image(
        new_width,
        new_height,
        &SRC_FORMAT,
        None,
        &[&new_data],
        &DST_FORMAT,
        None,
        &mut [&mut y_data, &mut u_data, &mut v_data],
    )?;

    panic!("odd sized resolutions not supported");

    Ok(vec![])
}


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
