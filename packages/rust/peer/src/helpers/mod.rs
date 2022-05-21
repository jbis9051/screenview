mod anti_replay;
pub mod cipher_reliable_peer;
pub mod cipher_unreliable_peer;
pub mod clipboard_type_map;
pub mod crypto;
pub mod native_thumbnails;
pub mod network_mouse_button_to_native;
pub mod rvd_native_helper;

// Differs from the spec currently, but we give ourselves a larger margin because of
// multithreading consistency concerns
pub const MAX_NONCE: u64 = i64::MAX as u64;