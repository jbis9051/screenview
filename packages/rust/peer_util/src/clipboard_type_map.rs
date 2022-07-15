use common::messages::rvd::ClipboardType as ClipboardNetwork;
use native::api::ClipboardType as ClipboardNative;

pub fn get_native_clipboard(network: &ClipboardNetwork) -> ClipboardNative {
    match network {
        ClipboardNetwork::Text => ClipboardNative::Text,
        ClipboardNetwork::Custom(str) => ClipboardNative::Custom(str.clone()),
        _ => panic!("map map from network clipboard to native"),
    }
}
