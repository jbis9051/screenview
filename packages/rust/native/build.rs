use cfg_if::cfg_if;

fn main() {
    cfg_if! {
        if #[cfg(dummy_native)] {
        } else if #[cfg(target_os="linux")] {
            println!("cargo:rustc-link-lib=X11");
        } else if #[cfg(windows)] {
        } else if #[cfg(target_os="macos")] {
        }
    }
}
