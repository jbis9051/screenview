use crate::{
    capture::{CapturePool, FrameProcessResult, ProcessFrame, ViewResources},
    rvd::Display,
};
use common::{messages::rvd::DisplayId, sync::event_loop::ThreadWaker};
use dcv_color_primitives as dcp;
use dcv_color_primitives::{convert_image, get_buffers_size, ColorSpace, ImageFormat, PixelFormat};
use image::{imageops::FilterType, DynamicImage, ImageFormat as ImageCrateFormat, RgbImage};
use native::{
    api::{BGRAFrame, NativeApiTemplate},
    NativeApi,
    NativeApiError,
};


pub struct ThumbnailCapture {
    pool: CapturePool<ProcessThumbnail>,
    captures: Vec<ThumbnailData>,
}

impl ThumbnailCapture {
    pub fn new(mut native: NativeApi, waker: ThreadWaker) -> Result<Self, NativeApiError> {
        let monitors = native.monitors()?;
        let windows = native.windows()?;

        let mut captures = Vec::with_capacity(monitors.len() + windows.len());
        captures.extend(monitors.into_iter().map(|monitor| ThumbnailData {
            name: monitor.name,
            display: Display::Monitor(monitor.id),
        }));
        captures.extend(windows.into_iter().map(|window| ThumbnailData {
            name: window.name,
            display: Display::Window(window.id),
        }));

        let mut pool = CapturePool::new(waker);

        for (index, capture) in captures.iter().enumerate().take(256) {
            pool.get_or_create_inactive()?
                .activate(capture.display, index as u8);
        }

        Ok(Self { pool, captures })
    }

    pub fn handle_thumbnail_updates<F>(&mut self, mut handler: F)
    where F: FnMut(NativeThumbnail) {
        for capture in self.pool.active_captures() {
            let update = match capture.next_update() {
                Some(update) => update,
                None => continue,
            };

            let raw = update.frame_update();
            let data = self
                .captures
                .get(raw.id)
                .expect("invalid or stale thumbnail id");
            handler(NativeThumbnail {
                data: raw.data.into(),
                name: data.name.clone(),
                display: data.display,
            });

            capture.update(update.resources);
        }
    }
}

#[derive(Default)]
struct ProcessThumbnail;

impl ProcessFrame for ProcessThumbnail {
    type Resources = Vec<u8>;

    fn process(
        &mut self,
        frame: &mut BGRAFrame,
        resources: &mut Self::Resources,
    ) -> FrameProcessResult {
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

        if get_buffers_size(frame.width, frame.height, &dst_format, None, sizes).is_err() {
            return FrameProcessResult::Failure;
        }

        let mut rgb_image = vec![0u8; sizes[0]];

        if convert_image(
            frame.width,
            frame.height,
            &src_format,
            None,
            &[&frame.data],
            &dst_format,
            None,
            &mut [&mut rgb_image],
        )
        .is_err()
        {
            return FrameProcessResult::Failure;
        }


        resources.clear();
        // TODO this can be sped up close to 10x using the resize library for this operation
        let result = DynamicImage::ImageRgb8(
            RgbImage::from_raw(frame.width, frame.height, rgb_image).unwrap(),
        )
        .resize(200, 200, FilterType::Nearest)
        .write_to(resources, ImageCrateFormat::Jpeg);

        if result.is_ok() {
            FrameProcessResult::Success
        } else {
            FrameProcessResult::Failure
        }
    }
}

impl<'a> ViewResources<'a> for ProcessThumbnail {
    type FrameUpdate = RawThumbnailData<'a>;
    type Resources = <Self as ProcessFrame>::Resources;

    fn to_frame_update(
        resources: &'a Self::Resources,
        _frame: &'a BGRAFrame,
        display_id: DisplayId,
    ) -> Self::FrameUpdate {
        RawThumbnailData {
            data: resources,
            id: usize::from(display_id),
        }
    }
}

pub struct RawThumbnailData<'a> {
    data: &'a [u8],
    id: usize,
}

pub struct NativeThumbnail {
    pub data: Box<[u8]>,
    pub name: String,
    pub display: Display,
}

struct ThumbnailData {
    name: String,
    display: Display,
}
