use std::thread::sleep;
use std::time::Duration;

use cfg_if::cfg_if;
use neon::prelude::*;

use api::*;

use crate::keymaps::keysym::{XK_Shift_L, XK_a};

mod api;
mod keymaps;
cfg_if! {
    if #[cfg(target_os="linux")] {
        mod unix;
        pub use unix::X11Api as NativeApi;
    } else if #[cfg(windows)] {
        mod windows;
        pub use windows::WindowsApi as NativeApi;
    } else if #[cfg(target_os="macos")] {
        mod mac;
        pub use mac::MacApi as NativeApi;
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

    cx.export_function("list_monitors", |mut cx| {
        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        let start = Instant::now();
        let windows = handle.borrow_mut().monitors().unwrap();
        let elapsed = start.elapsed();
        println!("{:?}", elapsed);
        println!("{:#?}", windows);
        Ok(cx.undefined())
    })?;

    cx.export_function("capture", |mut cx| {
        use image::ImageFormat;
        use std::time::Instant;
        for i in 0..20 {
            let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
            let mut h = handle.borrow_mut();
            let m = h.monitors().unwrap();
            println!("{:?}", m);
            let start = Instant::now();
            let img = h.capture_display_frame(&m[0]).unwrap();
            let elapsed = start.elapsed();
            println!("Time elapsed: {:?}", elapsed);
            if i == 19 {
                img.save_with_format("./cap.png", ImageFormat::Png).unwrap();
            }
        }
        Ok(cx.undefined())
    })?;

    cx.export_function("movemouse", |mut cx| {
        let handle = cx.argument::<JsBox<NativeApi>>(0)?;
        let start = Instant::now();
        handle
            .set_pointer_position(MousePosition {
                x: 500,
                y: 500,
                monitor_id: 0,
            })
            .unwrap();
        let elapsed = start.elapsed();
        println!("Time elapsed: {:?}", elapsed);
        Ok(cx.undefined())
    })?;

    cx.export_function("pointer_position", |mut cx| {
        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        loop {
            let start = Instant::now();
            let position = handle.borrow_mut().pointer_position().unwrap();
            println!("{:?}", position);
            let elapsed = start.elapsed();
            println!("{:?}", elapsed);
            sleep(Duration::from_millis(500));
        }
        Ok(cx.undefined())
    })?;

    cx.export_function("key_toggle", |mut cx| {
        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        let start = Instant::now(); //
        handle.borrow_mut().key_toggle(XK_Shift_L, true);
        handle.borrow_mut().key_toggle(XK_a, true);
        handle.borrow_mut().key_toggle(XK_a, false);
        handle.borrow_mut().key_toggle(XK_Shift_L, false);
        handle.borrow_mut().key_toggle(XK_a, true);
        handle.borrow_mut().key_toggle(XK_a, false);
        let elapsed = start.elapsed();
        println!("{:?}", elapsed);
        Ok(cx.undefined())
    })?;

    cx.export_function("set_pointer_position", |mut cx| {
        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        for i in 0..20 {
            let start = Instant::now(); //
            handle.borrow_mut().set_pointer_position(MousePosition {
                x: 10 * i,
                y: 10 * i,
                monitor_id: 0,
            });
            let elapsed = start.elapsed();
            println!("{:?}", elapsed);
            sleep(Duration::from_millis(500));
        }
        Ok(cx.undefined())
    })?;

    cx.export_function("toggle_mouse", |mut cx| {
        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        for _i in 0..20 {
            let start = Instant::now(); //
            handle
                .borrow_mut()
                .toggle_mouse(MouseButton::ScrollUp, false);
            let elapsed = start.elapsed();
            println!("{:?}", elapsed);
            sleep(Duration::from_millis(500));
        }
        Ok(cx.undefined())
    })?;

    cx.export_function("clipboard_content", |mut cx| {
        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        let start = Instant::now();
        println!(
            "{:?}",
            String::from_utf8(
                handle
                    .borrow_mut()
                    .clipboard_content(ClipboardType::Text)
                    .unwrap()
            )
            .unwrap()
        );
        let elapsed = start.elapsed();
        println!("{:?}", elapsed);
        sleep(Duration::from_millis(500));
        Ok(cx.undefined())
    })?;

    cx.export_function("set_clipboard_content", |mut cx| {
        let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
        let start = Instant::now();
        handle
            .borrow_mut()
            .set_clipboard_content(ClipboardType::Text, b"YELLOWSUBMARINE")
            .unwrap();
        let elapsed = start.elapsed();
        println!("{:?}", elapsed);
        sleep(Duration::from_millis(500));
        Ok(cx.undefined())
    })?;

    cx.export_function("capture_window", |mut cx| {
        use image::ImageFormat;
        use std::time::Instant;
        for i in 0..20 {
            let handle = cx.argument::<JsBox<RefCell<NativeApi>>>(0)?;
            let mut h = handle.borrow_mut();
            let w = h.windows().unwrap();
            let chosen = &w[5];
            println!("{:?}", chosen);
            let start = Instant::now();
            let img = h.capture_window_frame(chosen).unwrap();
            let elapsed = start.elapsed();
            println!("Time elapsed: {:?}", elapsed);
            if i == 19 {
                img.save_with_format("./cap_w.png", ImageFormat::Png).unwrap();
            }
        }
        Ok(cx.undefined())
    })?;

    Ok(())
}
