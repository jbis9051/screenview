use crate::rvd::DisplayType;
use image::{imageops::FilterType, DynamicImage, ImageError, ImageFormat};
use native::api::{Frame, NativeApiTemplate};

pub struct NativeThumbnail {
    pub data: Vec<u8>,
    pub name: String,
    pub native_id: u32,
    pub display_type: DisplayType,
}

fn process_frame<T: NativeApiTemplate>(frame: Frame) -> Result<Vec<u8>, ThumbnailError<T>> {
    let mut bytes = Vec::new();

    DynamicImage::ImageRgb8(frame)
        .resize(300, 300, FilterType::CatmullRom)
        .write_to(&mut bytes, ImageFormat::Jpeg)
        .map_err(ThumbnailError::Image)?;

    Ok(bytes)
}

pub fn native_thumbnails<T: NativeApiTemplate>(
    native: &mut T,
) -> Result<Vec<NativeThumbnail>, ThumbnailError<T>> {
    let mut thumbs = Vec::new();
    let windows = native.windows().map_err(ThumbnailError::Native)?;
    let monitors = native.monitors().map_err(ThumbnailError::Native)?;

    for window in windows {
        let frame = {
            match native.capture_window_frame(window.id) {
                Ok(frame) => frame,
                Err(_) => continue,
            }
        };

        let bytes = process_frame(frame)?;

        thumbs.push(NativeThumbnail {
            data: bytes.to_vec(),
            name: window.name,
            native_id: window.id,
            display_type: DisplayType::Window,
        });
    }

    for monitor in monitors {
        let frame = {
            match native.capture_monitor_frame(monitor.id) {
                Ok(frame) => frame,
                Err(_) => continue,
            }
        };

        let bytes = process_frame(frame)?;

        thumbs.push(NativeThumbnail {
            data: bytes,
            name: monitor.name,
            native_id: monitor.id,
            display_type: DisplayType::Monitor,
        });
    }

    Ok(thumbs)
}

#[derive(thiserror::Error, Debug)]
pub enum ThumbnailError<T: NativeApiTemplate> {
    #[error("Native error: {0:?}")]
    Native(T::Error),
    #[error("Image error: {0}")]
    Image(ImageError),
}
