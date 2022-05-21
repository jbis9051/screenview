use std::{thread::JoinHandle, time::Instant};

use common::{event_loop::ThreadWaker, messages::rvd::DisplayId};
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use native::{api::NativeApiTemplate, NativeApi, NativeApiError};
use std::{mem, thread, time::Duration};

use crate::rvd::Display;

use super::{
    processing::{ProcessFrame, ProcessorResources},
    CaptureResources,
};

const BROKEN_PIPE_MSG: &str = "broken pipe in frame capture";
const FPS: Duration = Duration::from_millis(50);

pub struct FrameCapture<P: ProcessFrame> {
    state: FrameCaptureState<P>,
}

impl<P: ProcessFrame> FrameCapture<P> {
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
                    .send(WorkerRequest::UpdateFrame(
                        Box::new(CaptureResources::new()),
                    ))
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

    pub fn update(&self, resources: Box<CaptureResources<P>>) {
        match &self.state {
            FrameCaptureState::Active { sender, .. } => {
                sender
                    .send(WorkerRequest::UpdateFrame(resources))
                    .expect(BROKEN_PIPE_MSG);
            }
            FrameCaptureState::Inactive { .. } =>
                panic!("Cannot update frame while in inactive state"),
        }
    }

    pub fn next_update(&mut self) -> Option<FrameUpdateResult<P>> {
        match &mut self.state {
            FrameCaptureState::Active {
                display_id,
                receiver,
                ..
            } => match receiver.try_recv() {
                Ok((resources, result)) => Some(FrameUpdateResult {
                    resources,
                    display_id: *display_id,
                    result,
                }),
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
        sender: Sender<(Box<CaptureResources<P>>, Result<(), NativeApiError>)>,
        receiver: Receiver<WorkerRequest<P>>,
    ) -> JoinHandle<(NativeApi, ThreadWaker)> {
        thread::spawn(move || Self::capture_frames(native_api, waker, display, sender, receiver))
    }

    fn capture_frames(
        mut native_api: NativeApi,
        waker: ThreadWaker,
        display: Display,
        sender: Sender<(Box<CaptureResources<P>>, Result<(), NativeApiError>)>,
        receiver: Receiver<WorkerRequest<P>>,
    ) -> (NativeApi, ThreadWaker) {
        let mut frame_processor = P::default();

        loop {
            let start = Instant::now();

            let mut resources = match receiver.recv().expect(BROKEN_PIPE_MSG) {
                WorkerRequest::UpdateFrame(resources) => resources,
                WorkerRequest::Stop => break,
            };

            let result = match display {
                Display::Monitor(id) => native_api.update_monitor_frame(id, resources.frame_mut()),
                Display::Window(id) => native_api.update_window_frame(id, resources.frame_mut()),
            };

            if result.is_ok() {
                frame_processor.process(&mut *resources);
            }

            sender.send((resources, result)).expect(BROKEN_PIPE_MSG);
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

impl<P: ProcessFrame> Drop for FrameCapture<P> {
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

enum FrameCaptureState<P: ProcessFrame> {
    Inactive {
        native_api: NativeApi,
        waker: ThreadWaker,
    },
    Active {
        display_id: DisplayId,
        sender: Sender<WorkerRequest<P>>,
        receiver: Receiver<(Box<CaptureResources<P>>, Result<(), NativeApiError>)>,
        handle: Option<JoinHandle<(NativeApi, ThreadWaker)>>,
    },
}

enum WorkerRequest<P: ProcessFrame> {
    UpdateFrame(Box<CaptureResources<P>>),
    Stop,
}

pub struct FrameUpdateResult<P: ProcessFrame> {
    pub resources: Box<CaptureResources<P>>,
    pub display_id: DisplayId,
    pub result: Result<(), NativeApiError>,
}

impl<P: ProcessFrame> FrameUpdateResult<P> {
    pub fn frame_update(&self) -> <P::Resources as ProcessorResources<'_>>::FrameUpdate {
        self.resources.frame_update(self.display_id)
    }
}
