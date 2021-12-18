mod api;

use api::*;
use cfg_if::cfg_if;
use image::RgbImage;
use neon::prelude::*;

cfg_if! {
    if #[cfg(target_os="linux")] {
        mod unix;
        pub use unix::*;
    } else if #[cfg(windows)] {
        compile_error!("Windows not supported yet");
    } else if #[cfg(any(target_os="ios", target_os="macos"))] {
        compile_error!("MacOS/iOS not supported yet");
    } else {
        compile_error!("Unknown target operating system");
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    use std::{cell::RefCell, time::Instant};
    struct JsCompat<T>(pub RefCell<T>);

    impl<T> Finalize for JsCompat<T> {}

    cx.export_function("new_handle", |mut cx| {
        Ok(JsBox::new(&mut cx, Capture::new().unwrap()).upcast::<JsValue>())
    })?;

    cx.export_function("capture", |mut cx| {
        use image::ImageFormat;
        use std::time::Instant;

        let handle = cx.argument::<JsBox<Capture>>(0)?;
        let start = Instant::now();
        let img = handle.capture_screen().unwrap();
        let elapsed = start.elapsed();
        println!("{}", elapsed.as_micros());

        img.save_with_format("./cap.png", ImageFormat::Png).unwrap();

        Ok(JsBox::new(&mut cx, JsCompat(RefCell::new(img))))
    })?;

    cx.export_function("update", |mut cx| {
        use image::ImageFormat;
        use std::time::Instant;

        let handle = cx.argument::<JsBox<Capture>>(0)?;
        let img_handle = cx.argument::<JsBox<JsCompat<RgbImage>>>(1)?;
        let mut img = img_handle.0.borrow_mut();

        let start = Instant::now();
        handle.update_screen_capture(&mut *img).unwrap();
        let elapsed = start.elapsed();
        println!("{}", elapsed.as_micros());

        img.save_with_format("./cap.png", ImageFormat::Png).unwrap();

        Ok(cx.undefined())
    })?;

    cx.export_function("keypress", |mut cx| {
        let handle = cx.argument::<JsBox<Capture>>(0)?;
        let start = Instant::now();
        handle.key_toggle('a' as _, true);
        handle.key_toggle('a' as _, false);
        let elapsed = start.elapsed();
        println!("Time elapsed: {:?}", elapsed);
        Ok(cx.undefined())
    })?;

    Ok(())
}
