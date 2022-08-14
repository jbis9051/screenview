// an InstanceHandler is the actual thing returned to node when they call new_instance (JsBox'ed)
use crate::{
    callback_interface::NodeInterface,
    instance::Instance,
    instance_main::{start_instance_main, Events},
    protocol::Message,
    screenview_handler::ScreenViewHandler,
};
use capture::CapturePool;
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use event_loop::{event_loop::ThreadWaker, oneshot, JoinOnDrop};
use native::{NativeApi, NativeApiError};
use neon::prelude::*;
use std::thread::JoinHandle;

pub struct InstanceHandle {
    sender: Sender<Message>,
    waker: ThreadWaker,
    _thread_handle: JoinOnDrop<()>,
}

impl InstanceHandle {
    fn new(sender: Sender<Message>, waker: ThreadWaker, thread_handle: JoinHandle<()>) -> Self {
        Self {
            sender,
            waker,
            _thread_handle: JoinOnDrop::new(thread_handle),
        }
    }

    pub fn send(&self, message: Message) -> bool {
        let res = self.sender.send(message).is_ok();
        self.waker.wake();
        res
    }

    fn new_with(
        channel: Channel,
        sv_handler: ScreenViewHandler,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        let (waker_tx, waker_rx) = oneshot::channel();
        let (message_tx, message_rx) = unbounded();
        let native = NativeApi::new()?;

        let thread_handle = start_instance_main(
            move |waker_core| {
                let instance = Instance {
                    native,
                    sv_handler,
                    callback_interface: node_interface,
                    capture_pool: CapturePool::new(
                        waker_core.make_waker(Events::FrameUpdate as u32),
                    ),
                    channel,
                    shared_displays: Default::default(),
                    decoders: Default::default(),
                    auth_schemes: Default::default(),
                    password: None,
                };

                waker_tx
                    .send(waker_core.make_waker(Events::InteropMessage as u32))
                    .unwrap();
                instance
            },
            message_rx,
        );

        Ok(InstanceHandle::new(
            message_tx,
            waker_rx.recv().unwrap(),
            thread_handle,
        ))
    }

    pub fn new_host_signal(
        channel: Channel,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(
            channel,
            ScreenViewHandler::new_host_signal(),
            node_interface,
        )
    }

    pub fn new_host_direct(
        channel: Channel,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(
            channel,
            ScreenViewHandler::new_host_direct(),
            node_interface,
        )
    }

    pub fn new_client_signal(
        channel: Channel,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(
            channel,
            ScreenViewHandler::new_client_signal(),
            node_interface,
        )
    }

    pub fn new_client_direct(
        channel: Channel,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(
            channel,
            ScreenViewHandler::new_client_direct(),
            node_interface,
        )
    }
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Shutdown);
        self.waker.wake();
        // thread_handle is dropped last
    }
}

impl Finalize for InstanceHandle {}
