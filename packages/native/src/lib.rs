mod api;

use api::*;
use cfg_if::cfg_if;
use neon::prelude::*;

cfg_if! {
    if #[cfg(target_os="linux")] {
        mod unix;
        pub use unix::X11Api as NativeApi;
    } else if #[cfg(windows)] {
        mod windows;
        pub use windows::WindowsApi as NativeApi;
    } else if #[cfg(any(target_os="ios", target_os="macos"))] {
        mod macos;
        pub use macos::MacosApi as NativeApi;
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
        Ok(JsBox::new(&mut cx, RefCell::new(NativeApi::new().unwrap())).upcast::<JsValue>())
    })?;

    cx.export_function("list_windows", |mut cx| {
        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        let start = Instant::now();
        let windows = handle.borrow_mut().windows().unwrap();
        let elapsed = start.elapsed();
        println!("{:?}", elapsed);
        println!("{:#?}", windows);
        Ok(cx.undefined())
    })?;

    cx.export_function("capture", |mut cx| {
        use image::ImageFormat;
        use std::time::Instant;

        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        let mut h = handle.borrow_mut();
        let start = Instant::now();
        let m = h.monitors().unwrap();
        let img = h.capture_display_frame(&m[0]).unwrap();
        let elapsed = start.elapsed();
        println!("{}", elapsed.as_micros());

        img.save_with_format("./cap.png", ImageFormat::Png).unwrap();

        Ok(JsBox::new(&mut cx, JsCompat(RefCell::new(img))))
    })?;

    cx.export_function("movemouse", |mut cx| {
        let handle = cx.argument::<JsBox<NativeApi>>(0)?;
        let start = Instant::now();
        handle
            .set_pointer_position(MousePosition { x: 500, y: 500 })
            .unwrap();
        let elapsed = start.elapsed();
        println!("Time elapsed: {:?}", elapsed);
        Ok(cx.undefined())
    })?;

    Ok(())
}
