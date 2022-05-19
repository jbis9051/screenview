use std::{thread::JoinHandle, time::Instant};

use common::{event_loop::ThreadWaker, messages::rvd::DisplayId};
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use native::{
    api::{Frame, NativeApiTemplate},
    NativeApi,
    NativeApiError,
};
use std::{mem, thread, time::Duration};

use crate::protocol::Display;

const BROKEN_PIPE_MSG: &str = "broken pipe in frame capture";
const FPS: Duration = Duration::from_millis(50);

pub struct FrameCapture {
    state: FrameCaptureState,
}

impl FrameCapture {
    pub fn new(waker: ThreadWaker) -> Result<Self, NativeApiError> {
        Ok(Self {
            state: FrameCaptureState::Inactive {
                native_api: NativeApi::new()?,
                waker,
            },
        })
    }

    pub fn activate(&mut self, display: Display, display_id: DisplayId) {
        let (request_sender, request_receiver) = bounded(1);
        let (response_sender, response_receiver) = bounded(1);

        let old_state = mem::replace(&mut self.state, FrameCaptureState::Active {
            display_id,
            frame_num: 0,
            sender: request_sender,
            receiver: response_receiver,
            handle: None,
        });

        let new_handle = match old_state {
            FrameCaptureState::Inactive { native_api, waker } => Self::start_worker_thread(
                native_api,
                waker,
                display,
                response_sender,
                request_receiver,
            ),
            // FIXME: when `self` is dropped, the unwrap on `handle` will panic causing an abort
            FrameCaptureState::Active { .. } =>
                panic!("Cannot activate frame capture when already in an active state"),
        };

        match &mut self.state {
            FrameCaptureState::Active { sender, handle, .. } => {
                sender
                    .send(WorkerRequest::UpdateFrame(Frame::new(0, 0)))
                    .expect(BROKEN_PIPE_MSG);
                *handle = Some(new_handle);
            }
            FrameCaptureState::Inactive { .. } => unreachable!(),
        }
    }

    pub fn is_capturing(&self, display_id: DisplayId) -> bool {
        match &self.state {
            &FrameCaptureState::Active {
                display_id: cur_display_id,
                ..
            } => display_id == cur_display_id,
            FrameCaptureState::Inactive { .. } => false,
        }
    }

    pub fn deactivate(&mut self) {
        let (native_api, waker) = match &mut self.state {
            FrameCaptureState::Active { handle, .. } => {
                handle
                    .take()
                    .expect("frame capture thread handle not present")
                    .join()
                    .unwrap() // Propagate panic if the worker thread panics
            }
            FrameCaptureState::Inactive { .. } =>
                panic!("Cannot deactivate frame capture when already in an inactive state"),
        };

        self.state = FrameCaptureState::Inactive { native_api, waker };
    }

    pub fn update(&self, frame: Frame) {
        match &self.state {
            FrameCaptureState::Active { sender, .. } => {
                sender
                    .send(WorkerRequest::UpdateFrame(frame))
                    .expect(BROKEN_PIPE_MSG);
            }
            FrameCaptureState::Inactive { .. } =>
                panic!("Cannot update frame while in inactive state"),
        }
    }

    pub fn next_update(&mut self) -> Option<FrameUpdateResult> {
        match &mut self.state {
            FrameCaptureState::Active {
                display_id,
                frame_num,
                receiver,
                ..
            } => match receiver.try_recv() {
                Ok((frame, result)) => {
                    let current_frame_num = *frame_num;
                    *frame_num = frame_num.checked_add(1).expect("frame count overflowed");
                    Some(FrameUpdateResult {
                        frame,
                        display_id: *display_id,
                        frame_num: current_frame_num,
                        result,
                    })
                }
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => panic!("{}", BROKEN_PIPE_MSG),
            },
            FrameCaptureState::Inactive { .. } => None,
        }
    }

    fn start_worker_thread(
        native_api: NativeApi,
        waker: ThreadWaker,
        display: Display,
        sender: Sender<(Frame, Result<(), NativeApiError>)>,
        receiver: Receiver<WorkerRequest>,
    ) -> JoinHandle<(NativeApi, ThreadWaker)> {
        thread::spawn(move || Self::capture_frames(native_api, waker, display, sender, receiver))
    }

    fn capture_frames(
        mut native_api: NativeApi,
        waker: ThreadWaker,
        display: Display,
        sender: Sender<(Frame, Result<(), NativeApiError>)>,
        receiver: Receiver<WorkerRequest>,
    ) -> (NativeApi, ThreadWaker) {
        loop {
            let start = Instant::now();

            let mut frame = match receiver.recv().expect(BROKEN_PIPE_MSG) {
                WorkerRequest::UpdateFrame(frame) => frame,
                WorkerRequest::Stop => break,
            };

            let result = match display {
                Display::Monitor(id) => native_api.update_monitor_frame(id, &mut frame),
                Display::Window(id) => native_api.update_window_frame(id, &mut frame),
            };

            sender.send((frame, result)).expect(BROKEN_PIPE_MSG);
            waker.wake();

            let elapsed = start.elapsed();
            if let Some(remaining) = FPS.checked_sub(elapsed) {
                if !remaining.is_zero() {
                    thread::sleep(remaining);
                }
            }
        }

        waker.wake();
        (native_api, waker)
    }
}

impl Drop for FrameCapture {
    fn drop(&mut self) {
        match &mut self.state {
            FrameCaptureState::Active { sender, handle, .. } => {
                let _ = sender.send(WorkerRequest::Stop);
                let _ = handle
                    .take()
                    .expect("drop should only ever be called once")
                    .join();
            }
            FrameCaptureState::Inactive { .. } => {}
        }
    }
}

enum FrameCaptureState {
    Inactive {
        native_api: NativeApi,
        waker: ThreadWaker,
    },
    Active {
        display_id: DisplayId,
        frame_num: u32,
        sender: Sender<WorkerRequest>,
        receiver: Receiver<(Frame, Result<(), NativeApiError>)>,
        handle: Option<JoinHandle<(NativeApi, ThreadWaker)>>,
    },
}

enum WorkerRequest {
    UpdateFrame(Frame),
    Stop,
}

pub struct FrameUpdateResult {
    pub frame: Frame,
    pub display_id: DisplayId,
    pub frame_num: u32,
    pub result: Result<(), NativeApiError>,
}
