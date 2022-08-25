use native::{api::NativeApiTemplate, NativeApi};
use peer_util::{decoder::Decoder, encoder::Encoder};
use std::time::{Duration, SystemTime};
use webrtc_util::Marshal;

#[cfg(feature = "e2e")]
#[test]
pub fn e2e() {
    let attempts = 10;
    let mut native = NativeApi::new().expect("Failed to initialize native API");
    let monitors = native.monitors().expect("Failed to get monitors");
    let monitor = &monitors[0];

    for i in 0 .. attempts {
        println!("Attempt {} ------------", i);
        let mut encoder = Encoder::new(50);
        let mut decoder = Decoder::new().expect("Failed to initialize decoder");

        for _ in 0 .. 3 {
            let mut frame = native
                .capture_monitor_frame(monitor.id)
                .expect("Failed to capture monitor");
            let pkts = encoder.process(&mut frame).expect("Failed to encode frame");
            println!("Broken into {} packets", pkts.len());
            for pkt in pkts {
                let bytes = pkt.marshal().expect("Failed to marshal packet");
                decoder
                    .process(bytes.to_vec())
                    .expect("Failed to decode packet");
            }
        }
    }
}
