use capture::ProcessFrame;
use native::{
    api::{
        BGRAFrame,
        ClipboardType as NativeClipboardType,
        Key,
        Monitor,
        MonitorId,
        MouseButton,
        MousePosition,
        NativeApiTemplate,
        Window,
        WindowId,
    },
    NativeApi,
};
use peer_util::frame_processor::FrameProcessor;
use std::convert::Infallible;

#[derive(Debug)]
struct TesterNative {
    pub monitor: Monitor,
}

impl TesterNative {
    pub fn new() -> Self {
        TesterNative {
            monitor: Monitor {
                id: 1,
                name: "Mock Display 1".to_string(),
                width: 1000,
                height: 1000,
            },
        }
    }
}

impl NativeApiTemplate for TesterNative {
    type Error = Infallible;

    fn key_toggle(&mut self, key: Key, down: bool) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn pointer_position(&mut self, _windows: &[WindowId]) -> Result<MousePosition, Self::Error> {
        unimplemented!()
    }

    fn set_pointer_position_absolute(
        &mut self,
        x: u32,
        y: u32,
        _monitor_id: MonitorId,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn set_pointer_position_relative(
        &mut self,
        x: u32,
        y: u32,
        _window_id: WindowId,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn toggle_mouse(
        &mut self,
        button: MouseButton,
        down: bool,
        _window_id: Option<WindowId>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn clipboard_content(
        &mut self,
        _type_name: &NativeClipboardType,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        unimplemented!()
    }

    fn set_clipboard_content(
        &mut self,
        _type_name: &NativeClipboardType,
        content: &[u8],
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn monitors(&mut self) -> Result<Vec<Monitor>, Self::Error> {
        Ok(vec![self.monitor.clone()])
    }

    fn windows(&mut self) -> Result<Vec<Window>, Self::Error> {
        unimplemented!()
    }

    fn capture_monitor_frame(&mut self, _monitor_id: MonitorId) -> Result<BGRAFrame, Self::Error> {
        Ok(BGRAFrame {
            width: self.monitor.width,
            height: self.monitor.height,
            data: vec![0; (self.monitor.width * self.monitor.height * 4) as usize],
        })
    }

    fn capture_window_frame(&mut self, _window_id: WindowId) -> Result<BGRAFrame, Self::Error> {
        unimplemented!()
    }
}
#[test]
fn frame_processor_test() {
    let mut native = TesterNative::new();
    let monitors = native.monitors().unwrap();
    let monitor = monitors.first().unwrap();
    let mut frame = native.capture_monitor_frame(monitor.id).unwrap();
    let mut processor = FrameProcessor::new(1500);
    let mut packets = Vec::new();
    processor.process(&mut frame, &mut packets).unwrap();
    // so this isn't really guaranteed but I guess it's fine
    assert!(!packets.is_empty())
}
