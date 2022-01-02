use cfg_if::cfg_if;

mod api;
mod keymaps;

cfg_if! {
    if #[cfg(dummy_api)] {
        pub use api::dummy::DummyApi as NativeApi;
    } else if #[cfg(target_os="linux")] {
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
