use crate::protocol::{ConnectionType, Message, RequestContent};
use common::event_loop::{event_loop, EventLoopState, ThreadWaker};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use native::{NativeApi, NativeApiError};
use neon::{
    prelude::{Channel, Context, Finalize, JsUndefined},
    types::Deferred,
};
use peer::{
    io::{TcpHandle, UdpHandle},
    services::ScreenViewHandler,
};
use std::thread::{self, JoinHandle};

pub struct InstanceHandle {
    sender: Sender<Message>,
    waker: ThreadWaker,
    thread_handle: Option<JoinHandle<()>>,
}

impl InstanceHandle {
    fn new(sender: Sender<Message>, waker: ThreadWaker, thread_handle: JoinHandle<()>) -> Self {
        Self {
            sender,
            waker,
            thread_handle: Some(thread_handle),
        }
    }

    pub fn send(&self, message: Message) -> bool {
        let res = self.sender.send(message).is_ok();
        self.waker.wake();
        res
    }
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Shutdown);
        self.waker.wake();
        let _ = self.thread_handle.take().unwrap().join();
    }
}

impl Finalize for InstanceHandle {}

pub struct Instance {
    native: NativeApi,
    sv_handler: ScreenViewHandler<TcpHandle, UdpHandle>,
    channel: Channel,
    waker: ThreadWaker,
}

impl Instance {
    fn new_with<F>(channel: Channel, new_sv_handler: F) -> Result<InstanceHandle, NativeApiError>
    where F: FnOnce() -> ScreenViewHandler<TcpHandle, UdpHandle> {
        let (tx, rx) = unbounded();
        let waker = ThreadWaker::new_current_thread();
        let instance = Self {
            native: NativeApi::new()?,
            sv_handler: new_sv_handler(),
            channel,
            waker: waker.clone(),
        };
        let thread_handle = start_instance_main(instance, rx);

        Ok(InstanceHandle::new(tx, waker, thread_handle))
    }

    pub fn new_host(channel: Channel) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(channel, ScreenViewHandler::new_host)
    }

    pub fn new_client(channel: Channel) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(channel, ScreenViewHandler::new_client)
    }

    fn handle_node_request(
        &mut self,
        content: RequestContent,
        promise: Deferred,
    ) -> Result<(), anyhow::Error> {
        match content {
            RequestContent::Connect {
                ref addr,
                connection_type,
            } => self.handle_connect(addr, connection_type, promise),
        }
    }

    fn handle_connect(
        &mut self,
        addr: &str,
        connection_type: ConnectionType,
        promise: Deferred,
    ) -> Result<(), anyhow::Error> {
        let io_handle = self.sv_handler.io_handle();

        let result = match connection_type {
            ConnectionType::Reliable => io_handle.connect_reliable(addr, self.waker.clone()),
            ConnectionType::Unreliable => io_handle.connect_unreliable(addr, self.waker.clone()),
        };

        let result = result.map_err(|error| error.to_string());

        promise.settle_with::<JsUndefined, _>(&self.channel, move |mut cx| match result {
            Ok(()) => Ok(cx.undefined()),
            Err(error) => {
                let error = cx.error(error)?;
                cx.throw(error)
            }
        });

        Ok(())
    }
}

impl Finalize for Instance {}

fn start_instance_main(instance: Instance, message_receiver: Receiver<Message>) -> JoinHandle<()> {
    thread::spawn(move || instance_main(instance, message_receiver))
}

fn instance_main(mut instance: Instance, message_receiver: Receiver<Message>) {
    event_loop(instance.waker.clone(), move || {
        // Handle incoming messages from node
        match message_receiver.try_recv() {
            Ok(Message::Request { content, promise }) => {
                match instance.handle_node_request(content, promise) {
                    Ok(()) => { /* yay */ }
                    Err(error) => panic!("Internal error while handling node request: {}", error),
                }
            }
            Ok(Message::Shutdown) => return EventLoopState::Complete,
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return EventLoopState::Complete,
        }

        // Handle messages from remote party
        match instance.sv_handler.handle_next_message() {
            Some(Ok(events)) =>
                for event in events {
                    todo!("Handle event")
                },
            Some(Err(error)) => todo!("Handle this error properly: {}", error),
            None => {}
        }

        EventLoopState::Working
    });
}
