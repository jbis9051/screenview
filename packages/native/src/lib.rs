mod api;

use cfg_if::cfg_if;
use neon::prelude::*;
use api::*;

cfg_if! {
    if #[cfg(target_os="linux")] {
        mod unix;
        pub type NativeImage = unix::X11Image;
        pub type NativeScreenHandle = unix::X11ScreenHandle;
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
    cx.export_function("new_handle", |mut cx| {
        match NativeScreenHandle::new() {
            Ok(sh) => Ok(JsBox::new(&mut cx, sh).upcast::<JsValue>()),
            Err(_) => Ok(cx.null().upcast::<JsValue>())
        }
    })?;

    Ok(())
}
