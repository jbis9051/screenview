use capture::ProcessFrame;
use native::{api::NativeApiTemplate, NativeApi};
use peer_util::frame_processor::FrameProcessor;

#[test]
fn frame_processor_test() {
    let mut native = NativeApi::new().unwrap();
    let monitors = native.monitors().unwrap();
    let monitor = monitors.first().unwrap();
    let mut frame = native.capture_monitor_frame(monitor.id).unwrap();
    let mut processor = FrameProcessor::new(1500);
    let mut packets = Vec::new();
    processor.process(&mut frame, &mut packets).unwrap();
    // so this isn't really guaranteed but I guess it's fine
    assert!(!packets.is_empty())
}
