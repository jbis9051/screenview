use crate::{
    forward,
    handler::ScreenViewHandler,
    node_interface::NodeInterface,
    protocol::{ConnectionType, Message, RequestContent},
    throw,
};
use common::{
    messages::{
        rvd::ButtonsMask,
        svsc::{Cookie, LeaseId},
    },
    sync::{
        event_loop::{event_loop, EventLoopState, ThreadWaker, ThreadWakerCore},
        oneshot,
        JoinOnDrop,
    },
};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use native::{NativeApi, NativeApiError};
use neon::{
    prelude::{Channel, Context, Finalize, JsResult, JsUndefined, TaskContext, Value},
    types::Deferred,
};
use peer::{
    capture::{CapturePool, DefaultFrameProcessor, DisplayInfoStore},
    io::{DirectServer, TcpHandle},
    rvd::{Display, ShareDisplayResult},
};
use std::{
    net::TcpStream,
    thread::{self, JoinHandle},
};

#[repr(u32)]
enum Events {
    RemoteMessage,
    InteropMessage,
    DirectServerConnection,
    FrameUpdate,
}

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

// an instance represents an instance of a rust interop
// this can be of type Client or Host and Direct or Signal

pub struct Instance {
    native: NativeApi,
    sv_handler: ScreenViewHandler,
    node_interface: NodeInterface,
    capture_pool: CapturePool<DefaultFrameProcessor>,
    channel: Channel,
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
        let (waker_tx, waker_rx) = oneshot::channel();
        let (message_tx, message_rx) = unbounded();
        let native = NativeApi::new()?;
        let sv_handler = new_sv_handler();

        let thread_handle = start_instance_main(
            move |waker_core| {
                let instance = Self {
                    native,
                    sv_handler,
                    node_interface,
                    capture_pool: CapturePool::new(
                        waker_core.make_waker(Events::FrameUpdate as u32),
                    ),
                    channel,
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

    fn settle_with_result<E, F, V>(&self, promise: Deferred, result: Result<(), E>, ret: F)
    where
        E: ToString,
        F: FnOnce(TaskContext<'_>) -> JsResult<'_, V> + Send + 'static,
        V: Value,
    {
        let result = result.map_err(|error| error.to_string());

        promise.settle_with(&self.channel, move |mut cx| match result {
            Ok(()) => ret(cx),
            Err(error) => throw!(cx, error),
        });
    }

    fn undefined(mut cx: TaskContext<'_>) -> JsResult<'_, JsUndefined> {
        Ok(cx.undefined())
    }

    fn handle_node_request(
        &mut self,
        content: RequestContent,
        promise: Deferred,
        waker_core: &ThreadWakerCore,
    ) -> Result<(), anyhow::Error> {
        match content {
            RequestContent::Connect {
                ref addr,
                connection_type,
            } => self.handle_connect(promise, waker_core, addr, connection_type),
            RequestContent::StartServer { ref addr } =>
                self.handle_start_server(promise, waker_core, addr),
            RequestContent::EstablishSession { lease_id } =>
                self.handle_establish_session(promise, lease_id),
            RequestContent::ProcessPassword { password } =>
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
            RequestContent::LeaseRequest { cookie } => self.handle_lease_request(promise, cookie),
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
        waker_core: &ThreadWakerCore,
        addr: &str,
        connection_type: ConnectionType,
    ) -> Result<(), anyhow::Error> {
        let io_handle = self.sv_handler.io_handle();

        let waker = waker_core.make_waker(Events::RemoteMessage as u32);
        let result = match connection_type {
            ConnectionType::Reliable => io_handle.connect_reliable(addr, waker),
            ConnectionType::Unreliable => io_handle.connect_unreliable(addr, waker),
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

    fn handle_direct_connect(&mut self, waker_core: &ThreadWakerCore, stream: TcpStream) {
        let io_handle = self.sv_handler.io_handle();

        // Terminate the connection
        if io_handle.is_reliable_connected() {
            drop(stream);
            return;
        }

        let waker = waker_core.make_waker(Events::RemoteMessage as u32);
        io_handle.connect_reliable_with(move |result_sender| {
            TcpHandle::new_from(stream, result_sender, waker)
        });
    }

    fn handle_start_server(
        &mut self,
        promise: Deferred,
        waker_core: &ThreadWakerCore,
        addr: &str,
    ) -> Result<(), anyhow::Error> {
        let server = match &mut self.sv_handler {
            ScreenViewHandler::HostDirect(_, server) => server,
            _ => unreachable!(),
        };

        if server.is_some() {
            promise.settle_with(&self.channel, |mut cx| -> JsResult<'_, JsUndefined> {
                throw!(
                    cx,
                    "Attempted to start server while one was already running"
                )
            });
            return Ok(());
        }

        let result = match DirectServer::new(
            addr,
            waker_core.make_waker(Events::DirectServerConnection as u32),
        ) {
            Ok(new_server) => {
                *server = Some(new_server);
                Ok(())
            }
            Err(error) => Err(error),
        };

        self.settle_with_result(promise, result, Self::undefined);

        Ok(())
    }

    fn handle_establish_session(
        &mut self,
        promise: Deferred,
        lease_id: LeaseId,
    ) -> Result<(), anyhow::Error> {
        let result = forward!(self.sv_handler, [HostSignal, ClientSignal], |stack| stack
            .establish_session_request(lease_id));
        self.settle_with_result(promise, result, Self::undefined);
        Ok(())
    }

    fn handle_process_password(
        &mut self,
        promise: Deferred,
        password: Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        let result = forward!(self.sv_handler, [ClientSignal, ClientDirect], |stack| stack
            .process_password(&password));
        self.settle_with_result(promise, result, Self::undefined);
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

    fn handle_lease_request(
        &mut self,
        promise: Deferred,
        cookie: Option<Cookie>,
    ) -> Result<(), anyhow::Error> {
        let result = forward!(self.sv_handler, [HostSignal, ClientSignal], |stack| stack
            .lease_request(cookie));
        self.settle_with_result(promise, result, Self::undefined);
        Ok(())
    }

    fn handle_update_static_password(
        &mut self,
        promise: Deferred,
        password: Option<Vec<u8>>,
    ) -> Result<(), anyhow::Error> {
        forward!(self.sv_handler, [HostSignal, HostDirect], |stack| stack
            .set_static_password(password));
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
        /*
         - Step 1: figure out which displays are not currently captured and need to be activated
         - Step 2: request monitor and window info from native which we will attempt to map onto
           the captured displays later
         - Step 3: find inactive frame captures which can be re-activated with a new display
         - Step 4: add new captures if necessary
         - Step 5: activate captures and record monitor/window info for a display change update
         - Step 6: report displays that couldn't be found and send display change message
        */

        let display_info = match DisplayInfoStore::new(&mut self.native) {
            Ok(store) => store,
            Err(error) => {
                self.settle_with_result(promise, Err(error), Self::undefined);
                return Ok(());
            }
        };

        let mut display_ids = Vec::with_capacity(displays.len());

        for &display in displays {
            let rvd_display = match display_info.gen_display_info(display) {
                Some(rvd_display) => rvd_display,
                None => todo!("tell node that we couldn't find this display"),
            };

            let id = forward!(self.sv_handler, [HostSignal, HostDirect], |stack| stack
                .share_display(rvd_display));
            match id {
                ShareDisplayResult::NewlyShared(display_id) =>
                    display_ids.push((display, display_id)),
                ShareDisplayResult::AlreadySharing(_) => {} // Ignore
                ShareDisplayResult::IdLimitReached =>
                    todo!("tell node that we ran out of display IDs"),
            }
        }

        let new_displays = display_ids
            .iter()
            .copied()
            .filter(|&(_, display_id)| !self.capture_pool.is_capturing(display_id))
            .collect::<Vec<_>>();
        let num_new_displays = new_displays.len();

        if num_new_displays == 0 {
            return Ok(());
        }

        for (display, display_id) in new_displays {
            let capture = match self.capture_pool.get_or_create_inactive() {
                Ok(capture) => capture,
                Err(_error) => todo!("tell node that we couldn't create a new capture"),
            };

            capture.activate(display, display_id);
        }

        let result = forward!(self.sv_handler, [HostSignal, HostDirect], |stack| stack
            .send_display_update());
        self.settle_with_result(promise, result, Self::undefined);

        Ok(())
    }
}

impl Finalize for Instance {}

fn start_instance_main<F>(make_instance: F, message_receiver: Receiver<Message>) -> JoinHandle<()>
where F: FnOnce(&ThreadWakerCore) -> Instance + Send + 'static {
    thread::spawn(move || {
        let waker_core = ThreadWakerCore::new_current_thread();
        let instance = make_instance(&waker_core);
        instance_main(waker_core, instance, message_receiver)
    })
}

fn instance_main(
    waker_core: ThreadWakerCore,
    mut instance: Instance,
    message_receiver: Receiver<Message>,
) {
    // TODO: do things with ThreadWaker and atomics so we know which channels to check

    event_loop(waker_core, move |waker_core| {
        // Handle incoming messages from node
        if waker_core.check_and_unset(Events::InteropMessage as u32) {
            // Infinite loop to clear the channel since node isn't a malicious party
            loop {
                match message_receiver.try_recv() {
                    Ok(Message::Request { content, promise }) => {
                        match instance.handle_node_request(content, promise, waker_core) {
                            Ok(()) => { /* yay */ }
                            Err(error) =>
                                panic!("Internal error while handling node request: {}", error),
                        }
                    }
                    Ok(Message::Shutdown) => return EventLoopState::Complete,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return EventLoopState::Complete,
                }
            }
        }

        // Handle messages from remote party
        if waker_core.check_and_unset(Events::RemoteMessage as u32) {
            const MAX_TO_HANDLE: usize = 8;
            let mut handled = 0;

            while handled < MAX_TO_HANDLE {
                match instance.sv_handler.handle_next_message() {
                    Some(Ok(events)) =>
                        for _event in events {
                            todo!("Handle event")
                        },
                    Some(Err(error)) => {
                        todo!("Handle this error properly: {}", error)
                    }
                    None => break,
                }

                handled += 1;
            }

            // We're receiving more remote messages than we can handle, but we don't want to block
            // the event loop too long, so change our state such that we continue handling remote
            // messages on the next iteration without parking
            if handled == MAX_TO_HANDLE {
                waker_core.wake_self(Events::RemoteMessage as u32);
            }
        }

        // If a server is running, handle incoming connections from there
        if waker_core.check_and_unset(Events::DirectServerConnection as u32) {
            // We don't need to put this in a loop due to the guarantee provided by `next_incoming`
            if let ScreenViewHandler::HostDirect(_, Some(ref server)) = instance.sv_handler {
                match server.next_incoming() {
                    Some(Ok(stream)) => {
                        // This stream should be blocking, however on macOS for some reason if the
                        // listener is non-blocking at the time of we accept a stream, the stream
                        // with also be non-blocking, so we explicitly set it to blocking
                        stream
                            .set_nonblocking(false)
                            .expect("Failed to set stream to blocking"); // TODO handle this better
                        instance.handle_direct_connect(waker_core, stream);
                    }
                    Some(Err(error)) => {
                        todo!("Handle this error properly: {}", error)
                    }
                    None => {}
                }
            }
        }

        if waker_core.check_and_unset(Events::FrameUpdate as u32) {
            // We don't need to double-loop here because the channels for frame capturing have a
            // capacity of 1, so we won't fall behind when processing frames, the capture threads
            // will just block and wait for us
            for capture in instance.capture_pool.active_captures() {
                let frame_update = match capture.next_update() {
                    Some(update) => update,
                    None => continue,
                };

                if let Err(_error) = frame_update.result {
                    todo!("Handle frame update errors properly");
                }

                let result = forward!(instance.sv_handler, [HostSignal, HostDirect], |stack| stack
                    .send_frame_update(frame_update.frame_update()));

                result.expect("handle errors from sending frame updates properly");

                capture.update(frame_update.resources);
            }
        }

        EventLoopState::Working
    });
}
