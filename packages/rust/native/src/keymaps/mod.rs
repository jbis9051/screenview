use cfg_if::cfg_if;

pub mod keysym;

cfg_if! {
    if #[cfg(dummy_native)] {
    } else if #[cfg(target_os = "macos")]{
        pub mod keycode_mac;
        pub mod keysym_to_mac;
    }
}
