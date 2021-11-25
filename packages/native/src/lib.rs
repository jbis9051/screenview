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
    use std::cell::RefCell;
    struct JsCompat<T>(pub RefCell<T>);

    impl<T> Finalize for JsCompat<T> {}

    cx.export_function("new_handle", |mut cx| {
        match ScreenHandle::new() {
            Ok(sh) => Ok(JsBox::new(&mut cx, sh).upcast::<JsValue>()),
            Err(_) => Ok(cx.null().upcast::<JsValue>())
        }
    })?;

    cx.export_function("capture", |mut cx| {
        use std::time::Instant;
        use image::ImageFormat;

        let handle = cx.argument::<JsBox<ScreenHandle>>(0)?;
        let start = Instant::now();
        let img = handle.capture().unwrap();
        let elapsed = start.elapsed();
        println!("{}", elapsed.as_micros());

        img.save_with_format("./cap.png", ImageFormat::Png).unwrap();

        Ok(JsBox::new(&mut cx, JsCompat(RefCell::new(img))))
    })?;

    cx.export_function("update", |mut cx| {
        use std::time::Instant;
        use image::ImageFormat;

        let handle = cx.argument::<JsBox<ScreenHandle>>(0)?;
        let img_handle = cx.argument::<JsBox<JsCompat<RgbImage>>>(1)?;
        let mut img = img_handle.0.borrow_mut();

        let start = Instant::now();
        handle.update(&mut *img).unwrap();
        let elapsed = start.elapsed();
        println!("{}", elapsed.as_micros());

        img.save_with_format("./cap.png", ImageFormat::Png).unwrap();

        Ok(cx.null())
    })?;

    Ok(())
}
