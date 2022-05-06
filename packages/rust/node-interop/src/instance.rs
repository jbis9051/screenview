use crate::{
    handler::ScreenViewHandler,
    protocol::{ConnectionType, Message, RequestContent},
};
use common::{
    event_loop::{event_loop, EventLoopState, JoinOnDrop, ThreadWaker},
    messages::rvd::ButtonsMask,
};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use native::{NativeApi, NativeApiError};
use neon::{
    prelude::{Channel, Context, Finalize},
    types::Deferred,
};
use std::thread::{self, JoinHandle};

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
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Shutdown);
        self.waker.wake();
        // thread_handle is dropped last
    }
}

impl Finalize for InstanceHandle {}

pub struct Instance {
    native: NativeApi,
    sv_handler: ScreenViewHandler,
    channel: Channel,
    waker: ThreadWaker,
}

impl Instance {
    fn new_with<F>(channel: Channel, new_sv_handler: F) -> Result<InstanceHandle, NativeApiError>
    where F: FnOnce() -> ScreenViewHandler {
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

    pub fn new_host_signal(channel: Channel) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(channel, ScreenViewHandler::new_host_signal)
    }

    pub fn new_host_direct(channel: Channel) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(channel, ScreenViewHandler::new_host_direct)
    }

    pub fn new_client_signal(channel: Channel) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(channel, ScreenViewHandler::new_client_signal)
    }

    pub fn new_client_direct(channel: Channel) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(channel, ScreenViewHandler::new_client_direct)
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
            RequestContent::EstablishSession { ref lease_id } =>
                self.handle_establish_session(lease_id, promise),
            RequestContent::ProcessPassword { ref password } =>
                self.handle_process_password(password, promise),
            RequestContent::MouseInput {
                x_position,
                y_position,
                button_mask,
                button_mask_state,
            } => self.handle_mouse_input(
                x_position,
                y_position,
                button_mask,
                button_mask_state,
                promise,
            ),
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

        promise.settle_with(&self.channel, move |mut cx| match result {
            Ok(()) => Ok(cx.undefined()),
            Err(error) => {
                let error = cx.error(error)?;
                cx.throw(error)
            }
        });

        Ok(())
    }

    fn handle_establish_session(
        &mut self,
        lease_id: &str,
        promise: Deferred,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_process_password(
        &mut self,
        password: &str,
        promise: Deferred,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_mouse_input(
        &mut self,
        x_position: i32,
        y_position: i32,
        button_mask: ButtonsMask,
        button_mask_state: ButtonsMask,
        promise: Deferred,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

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
