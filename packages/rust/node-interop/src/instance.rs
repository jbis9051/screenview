use crate::{
    handler::ScreenViewHandler,
    node_interface::NodeInterface,
    protocol::{ConnectionType, Display, Message, RequestContent},
    throw,
};
use common::{
    event_loop::{event_loop, EventLoopState, JoinOnDrop, ThreadWaker},
    messages::rvd::ButtonsMask,
};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use native::{NativeApi, NativeApiError};
use neon::{
    prelude::{Channel, Context, Finalize, JsResult, JsUndefined},
    types::Deferred,
};
use peer::io::{DirectServer, TcpHandle};
use std::{
    net::TcpStream,
    thread::{self, JoinHandle},
};

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
    node_interface: NodeInterface,
    server: Option<DirectServer>,
    channel: Channel,
    waker: ThreadWaker,
}

impl Instance {
    fn new_with<F>(
        channel: Channel,
        new_sv_handler: F,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError>
    where
        F: FnOnce() -> ScreenViewHandler,
    {
        let (tx, rx) = unbounded();
        let waker = ThreadWaker::new_current_thread();
        let instance = Self {
            native: NativeApi::new()?,
            sv_handler: new_sv_handler(),
            node_interface,
            server: None,
            channel,
            waker: waker.clone(),
        };
        let thread_handle = start_instance_main(instance, rx);

        Ok(InstanceHandle::new(tx, waker, thread_handle))
    }

    pub fn new_host_signal(
        channel: Channel,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(channel, ScreenViewHandler::new_host_signal, node_interface)
    }

    pub fn new_host_direct(
        channel: Channel,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(channel, ScreenViewHandler::new_host_direct, node_interface)
    }

    pub fn new_client_signal(
        channel: Channel,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(
            channel,
            ScreenViewHandler::new_client_signal,
            node_interface,
        )
    }

    pub fn new_client_direct(
        channel: Channel,
        node_interface: NodeInterface,
    ) -> Result<InstanceHandle, NativeApiError> {
        Self::new_with(
            channel,
            ScreenViewHandler::new_client_direct,
            node_interface,
        )
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
            } => self.handle_connect(promise, addr, connection_type),
            RequestContent::StartServer { ref addr } => self.handle_start_server(promise, addr),
            RequestContent::EstablishSession { lease_id } =>
                self.handle_establish_session(promise, lease_id),
            RequestContent::ProcessPassword { ref password } =>
                self.handle_process_password(promise, password),
            RequestContent::MouseInput {
                x_position,
                y_position,
                button_mask,
                button_mask_state,
            } => self.handle_mouse_input(
                promise,
                x_position,
                y_position,
                button_mask,
                button_mask_state,
            ),
            RequestContent::KeyboardInput { keycode, down } =>
                self.handle_keyboard_input(promise, keycode, down),
            RequestContent::LeaseRequest => self.handle_lease_request(promise),
            RequestContent::UpdateStaticPassword { password } =>
                self.handle_update_static_password(promise, password),
            RequestContent::SetControllable { is_controllable } =>
                self.handle_set_controllable(promise, is_controllable),
            RequestContent::SetClipboardReadable { is_readable } =>
                self.handle_set_clipboard_readable(promise, is_readable),
            RequestContent::ShareDisplays { ref displays } =>
                self.handle_share_displays(promise, displays),
        }
    }

    fn handle_connect(
        &mut self,
        promise: Deferred,
        addr: &str,
        connection_type: ConnectionType,
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

    fn handle_direct_connect(&mut self, stream: TcpStream) {
        let io_handle = self.sv_handler.io_handle();

        // Terminate the connection
        if io_handle.is_reliable_connected() {
            drop(stream);
            return;
        }

        let waker = self.waker.clone();
        io_handle.connect_reliable_with(move |result_sender| {
            TcpHandle::new_from(stream, result_sender, waker)
        });
    }

    fn handle_start_server(&mut self, promise: Deferred, addr: &str) -> Result<(), anyhow::Error> {
        if !matches!(self.sv_handler, ScreenViewHandler::HostDirect(..)) {
            panic!("Attempted to start server on non-host-direct instance");
        }

        if self.server.is_some() {
            promise.settle_with(&self.channel, |mut cx| -> JsResult<'_, JsUndefined> {
                throw!(
                    cx,
                    "Attempted to start server while one was already running"
                )
            });
            return Ok(());
        }

        let result = match DirectServer::new(addr, self.waker.clone()) {
            Ok(server) => {
                self.server = Some(server);
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        };

        promise.settle_with(&self.channel, move |mut cx| match result {
            Ok(()) => Ok(cx.undefined()),
            Err(error) => throw!(cx, error),
        });

        Ok(())
    }

    fn handle_establish_session(
        &mut self,
        promise: Deferred,
        lease_id: [u8; 4],
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_process_password(
        &mut self,
        promise: Deferred,
        password: &str,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_mouse_input(
        &mut self,
        promise: Deferred,
        x_position: i32,
        y_position: i32,
        button_mask: ButtonsMask,
        button_mask_state: ButtonsMask,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_keyboard_input(
        &mut self,
        promise: Deferred,
        keycode: u32,
        down: bool,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_lease_request(&mut self, promise: Deferred) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_update_static_password(
        &mut self,
        promise: Deferred,
        password: Option<String>,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_set_controllable(
        &mut self,
        promise: Deferred,
        is_controllable: bool,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_set_clipboard_readable(
        &mut self,
        promise: Deferred,
        is_readable: bool,
    ) -> Result<(), anyhow::Error> {
        // TODO: implement

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));

        Ok(())
    }

    fn handle_share_displays(
        &mut self,
        promise: Deferred,
        displays: &[Display],
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
        let mut state = EventLoopState::Waiting;

        // Handle incoming messages from node
        match message_receiver.try_recv() {
            Ok(Message::Request { content, promise }) => {
                match instance.handle_node_request(content, promise) {
                    Ok(()) => { /* yay */ }
                    Err(error) => panic!("Internal error while handling node request: {}", error),
                }

                state = EventLoopState::Working;
            }
            Ok(Message::Shutdown) => return EventLoopState::Complete,
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return EventLoopState::Complete,
        }

        // Handle messages from remote party
        match instance.sv_handler.handle_next_message() {
            Some(Ok(events)) => {
                for event in events {
                    todo!("Handle event")
                }

                state = EventLoopState::Working;
            }
            Some(Err(error)) => {
                state = EventLoopState::Working;

                todo!("Handle this error properly: {}", error)
            }
            None => {}
        }

        // If a server is running, handle incoming connections from there
        match instance.server.as_ref().and_then(|server| server.recv()) {
            Some(Ok(stream)) => {
                instance.handle_direct_connect(stream);
                state = EventLoopState::Working;
            }
            Some(Err(error)) => {
                state = EventLoopState::Working;

                todo!("Handle this error properly: {}", error)
            }
            None => {}
        }

        state
    });
}
