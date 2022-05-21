use crate::{
    forward,
    handler::ScreenViewHandler,
    node_interface::NodeInterface,
    protocol::{ConnectionType, Message, RequestContent},
    throw,
};
use common::{
    event_loop::{event_loop, EventLoopState, JoinOnDrop, ThreadWaker},
    messages::{
        rvd::ButtonsMask,
        svsc::{Cookie, LeaseId},
    },
};
use crossbeam_channel::{bounded, unbounded, Receiver, Sender, TryRecvError};
use native::{NativeApi, NativeApiError};
use neon::{
    object::Object,
    prelude::{Channel, Context, Finalize, JsResult, JsUndefined, TaskContext, Value},
    types::{Deferred, JsArray},
};
use peer::{
    capture::{CapturePool, DisplayInfoStore},
    helpers::native_thumbnails::native_thumbnails,
    io::{DirectServer, TcpHandle},
    rvd::{Display, DisplayType, ShareDisplayResult},
};
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

// an instance represents an instance of a rust interop
// this can be of type Client or Host and Direct or Signal

pub struct Instance {
    native: NativeApi,
    sv_handler: ScreenViewHandler,
    node_interface: NodeInterface,
    capture_pool: CapturePool,
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
        let (waker_tx, waker_rx) = bounded(1);
        let (message_tx, message_rx) = unbounded();
        let native = NativeApi::new()?;
        let sv_handler = new_sv_handler();

        let thread_handle = start_instance_main(
            move |waker| {
                let instance = Self {
                    native,
                    sv_handler,
                    node_interface,
                    capture_pool: CapturePool::new(waker.clone()),
                    channel,
                    waker: waker.clone(),
                };

                waker_tx.send(waker).unwrap();
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
    ) -> Result<(), anyhow::Error> {
        match content {
            RequestContent::Connect {
                ref addr,
                connection_type,
            } => self.handle_connect(promise, addr, connection_type),
            RequestContent::StartServer { ref addr } => self.handle_start_server(promise, addr),
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
            RequestContent::NativeThumbnails => self.handle_thumbnails(promise),
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

        let result = match DirectServer::new(addr, self.waker.clone()) {
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
            .process_password(password));
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

    fn handle_thumbnails(&mut self, promise: Deferred) -> Result<(), anyhow::Error> {
        let vec = match native_thumbnails(&mut self.native) {
            Ok(vec) => vec,
            Err(err) => {
                self.settle_with_result(promise, Err(err), Self::undefined);
                return Ok(());
            }
        };

        promise.settle_with(&self.channel, move |mut cx| {
            let array = JsArray::new(&mut cx, vec.len() as u32);

            for (i, thumb) in vec.iter().enumerate() {
                let obj = cx.empty_object();

                let data = cx.array_buffer(thumb.data.len())?;
                for (i, u8) in thumb.data.iter().enumerate() {
                    let num = cx.number(*u8 as f64);
                    data.set(&mut cx, i as u32, num)?;
                }

                obj.set(&mut cx, "data", data)?;
                let str = cx.string(&thumb.name);
                obj.set(&mut cx, "name", str)?;
                let num = cx.number(thumb.native_id as f64);
                obj.set(&mut cx, "native_id", num)?;

                let display_type = match thumb.display_type {
                    DisplayType::Window => "window",
                    DisplayType::Monitor => "monitor",
                };
                let str = cx.string(display_type);
                obj.set(&mut cx, "display_type", str)?;

                array.set(&mut cx, i as u32, obj)?;
            }

            Ok(array)
        });

        Ok(())
    }
}

impl Finalize for Instance {}

fn start_instance_main<F>(make_instance: F, message_receiver: Receiver<Message>) -> JoinHandle<()>
where F: FnOnce(ThreadWaker) -> Instance + Send + 'static {
    thread::spawn(move || {
        let waker = ThreadWaker::new_current_thread();
        let instance = make_instance(waker);
        instance_main(instance, message_receiver)
    })
}

fn instance_main(mut instance: Instance, message_receiver: Receiver<Message>) {
    // TODO: do things with ThreadWaker and atomics so we know which channels to check

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
                for _event in events {
                    todo!("Handle event")
                },
            Some(Err(error)) => {
                todo!("Handle this error properly: {}", error)
            }
            None => {}
        }

        // If a server is running, handle incoming connections from there
        if let ScreenViewHandler::HostDirect(_, Some(ref server)) = instance.sv_handler {
            match server.recv() {
                Some(Ok(stream)) => {
                    instance.handle_direct_connect(stream);
                }
                Some(Err(error)) => {
                    todo!("Handle this error properly: {}", error)
                }
                None => {}
            }
        }

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

        EventLoopState::Working
    });
}
