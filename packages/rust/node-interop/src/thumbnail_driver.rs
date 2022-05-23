use common::sync::{
    event_loop::{event_loop, EventLoopState, ThreadWaker, ThreadWakerCore},
    oneshot,
    JoinOnDrop,
};
use native::{NativeApi, NativeApiError};
use neon::prelude::*;
use peer::{helpers::native_thumbnails::ThumbnailCapture, rvd::Display};
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
};

#[repr(u32)]
enum Events {
    ThumbnailUpdate,
    Stop,
}

pub struct ThumbnailHandle {
    waker: ThreadWaker,
    _handle: JoinOnDrop<()>,
}

impl ThumbnailHandle {
    pub fn new(channel: Channel, callback: Root<JsFunction>) -> Result<Self, NativeApiError> {
        let (waker_sender, waker_receiver) = oneshot::channel();
        let driver_handle = start_driver_main(waker_sender, channel, Arc::new(callback));
        let waker = waker_receiver.recv().unwrap()?;

        Ok(Self {
            waker,
            _handle: JoinOnDrop::new(driver_handle),
        })
    }

    pub fn close(&self) {
        // Sets a flag that tells the event loop to stop
        self.waker.wake();
    }
}

impl Drop for ThumbnailHandle {
    fn drop(&mut self) {
        self.close();
    }
}

impl Finalize for ThumbnailHandle {}

fn start_driver_main(
    waker_sender: oneshot::Sender<Result<ThreadWaker, NativeApiError>>,
    channel: Channel,
    callback: Arc<Root<JsFunction>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let waker_core = ThreadWakerCore::new_current_thread();
        let capture_result = NativeApi::new().and_then(|api| {
            ThumbnailCapture::new(api, waker_core.make_waker(Events::ThumbnailUpdate as u32))
        });

        let capture = match capture_result {
            Ok(capture) => {
                waker_sender
                    .send(Ok(waker_core.make_waker(Events::Stop as u32)))
                    .unwrap();
                capture
            }
            Err(error) => {
                waker_sender.send(Err(error)).unwrap();
                return;
            }
        };

        driver_main(waker_core, capture, channel, callback);
    })
}

fn driver_main(
    waker_core: ThreadWakerCore,
    mut capture: ThumbnailCapture,
    channel: Channel,
    callback: Arc<Root<JsFunction>>,
) {
    event_loop(waker_core, move |waker_core| {
        if waker_core.check_and_unset(Events::ThumbnailUpdate as u32) {
            let mut updates = Vec::new();

            capture.handle_thumbnail_updates(|update| {
                updates.push(update);
            });

            channel.send({
                let callback = Arc::clone(&callback);
                move |mut cx| {
                    let array = JsArray::new(&mut cx, updates.len() as u32);

                    for (i, thumb) in updates.into_iter().enumerate() {
                        let obj = cx.empty_object();

                        let data = JsArrayBuffer::external(&mut cx, thumb.data);
                        obj.set(&mut cx, "data", data)?;

                        let str = cx.string(&thumb.name);
                        obj.set(&mut cx, "name", str)?;

                        let (display_id, display_type) = match thumb.display {
                            Display::Monitor(id) => (id, "monitor"),
                            Display::Window(id) => (id, "window"),
                        };

                        let num = cx.number(display_id as f64);
                        obj.set(&mut cx, "native_id", num)?;

                        let str = cx.string(display_type);
                        obj.set(&mut cx, "display_type", str)?;

                        array.set(&mut cx, i as u32, obj)?;
                    }

                    let this = cx.null();
                    let _ = callback
                        .to_inner(&mut cx)
                        .call(&mut cx, this, [array.upcast()]);
                    Ok(())
                }
            });
        }

        if waker_core.check_and_unset(Events::Stop as u32) {
            return EventLoopState::Complete;
        }

        EventLoopState::Working
    });
}
