use cfg_if::cfg_if;

pub mod api;
mod keymaps;

cfg_if! {
    if #[cfg(dummy_native)] {
        pub use api::dummy::DummyApi as NativeApi;
        pub use std::convert::Infallible as NativeApiError;
    } else if #[cfg(target_os="linux")] {
        mod unix;
        pub use unix::X11Api as NativeApi;
        pub use unix::Error as NativeApiError;
    } else if #[cfg(windows)] {
        mod windows;
        pub use windows::WindowsApi as NativeApi;
        pub use windows::Error as NativeApiError;
    } else if #[cfg(target_os="macos")] {
        mod mac;
        pub use mac::MacApi as NativeApi;
        pub use mac::Error as NativeApiError;
    } else {
        compile_error!("Unknown target operating system");
    }
}
