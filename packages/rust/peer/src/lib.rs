mod io;
pub mod services;

mod native {
    cfg_if::cfg_if! {
        if #[cfg(tests)] {
            pub use ::native::api;
            pub type NativeApiError = Box<dyn std::error::Error + 'static>;
            pub type NativeApi = Box<dyn api::NativeApiTemplate<Error = NativeApiError>>;
        } else {
            pub use ::native::*;
        }
    }
}
