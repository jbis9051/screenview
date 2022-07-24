// an instance represents an instance of a rust interop
// this can be of type Client or Host and Direct or Signal

use crate::{
    callback_interface::NodeInterface,
    forward,
    instance_main::Events,
    protocol::{ConnectionType, Message, RequestContent},
    screenview_handler::ScreenViewHandler,
    throw,
};
use capture::CapturePool;
use common::messages::{
    rvd::{AccessMask, ButtonsMask, DisplayId},
    svsc::{Cookie, LeaseId},
    wpskka::AuthSchemeType,
};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use event_loop::{
    event_loop::{event_loop, EventLoopState, ThreadWaker, ThreadWakerCore},
    oneshot,
    oneshot::channel,
    JoinOnDrop,
};
use io::{DirectServer, TcpHandle};
use native::{
    api::{NativeApiTemplate, NativeId},
    NativeApi,
    NativeApiError,
};
use neon::{
    prelude::{Channel, Context, Finalize, JsResult, JsUndefined, TaskContext, Value},
    types::Deferred,
};
use peer::{
    rvd::{RvdClientInform, RvdHostInform},
    svsc_handler::SvscInform,
    wpskka::{WpskkaClientInform, WpskkaHostInform},
    InformEvent,
};
use peer_util::{
    frame_processor::FrameProcessor,
    rvd_native_helper::{rvd_client_native_helper, rvd_host_native_helper},
};
use std::{
    collections::HashMap,
    net::TcpStream,
    thread::{self, JoinHandle},
};

pub struct Instance {
    pub(crate) native: NativeApi,
    pub(crate) sv_handler: ScreenViewHandler,
    pub(crate) callback_interface: NodeInterface,
    pub(crate) capture_pool: CapturePool<FrameProcessor>,
    pub(crate) channel: Channel,
    pub(crate) shared_displays: HashMap<DisplayId, NativeId>,
    pub(crate) auth_schemes: Vec<AuthSchemeType>,
    pub(crate) password: Option<String>,
}

impl Instance {
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

    pub(crate) fn handle_node_request(
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
            RequestContent::StartServer {
                ref reliable_addr,
                ref unreliable_addr,
            } => self.handle_start_server(promise, waker_core, reliable_addr, unreliable_addr),
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
            RequestContent::ShareDisplays {
                displays,
                controllable,
            } => self.handle_share_displays(promise, displays, controllable),
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
            ConnectionType::Unreliable =>
                io_handle.bind_and_connect_unreliable("0.0.0.0:0", addr, waker),
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

    pub(crate) fn handle_direct_connect(
        &mut self,
        waker_core: &ThreadWakerCore,
        stream: TcpStream,
    ) -> Result<(), anyhow::Error> {
        let io_handle = self.sv_handler.io_handle();

        // Terminate the connection
        if io_handle.is_reliable_connected() {
            drop(stream);
            return Ok(());
        }

        let waker = waker_core.make_waker(Events::RemoteMessage as u32);
        io_handle.connect_reliable_with(move |result_sender| {
            TcpHandle::new_from(stream, result_sender, waker)
        });

        forward!(self.sv_handler, [HostDirect], |stack| {
            stack.key_exchange()
        })
        .expect("unable to produce key exchange"); // TODO error handling

        Ok(())
    }

    fn handle_start_server(
        &mut self,
        promise: Deferred,
        waker_core: &ThreadWakerCore,
        reliable_addr: &str,
        unreliable_addr: &str,
    ) -> Result<(), anyhow::Error> {
        let (io_handle, server) = match &mut self.sv_handler {
            ScreenViewHandler::HostDirect(stack, server) => (&mut stack.io_handle, server),
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
            reliable_addr,
            waker_core.make_waker(Events::DirectServerConnection as u32),
        ) {
            Ok(new_server) => {
                *server = Some(new_server);
                io_handle.bind_unreliable(
                    unreliable_addr,
                    waker_core.make_waker(Events::RemoteMessage as u32),
                )
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
        displays: Vec<NativeId>,
        controllable: bool,
    ) -> Result<(), anyhow::Error> {
        // Get stuff to unsshare
        let to_unshare: Vec<_> = self
            .shared_displays
            .iter()
            .filter(|(_, native)| !displays.iter().any(|native1| native1 == *native))
            .map(|(display_id, _)| *display_id)
            .collect();

        // Unshare them
        for display_id in to_unshare {
            self.shared_displays.remove(&display_id);
            forward!(self.sv_handler, [HostSignal, HostDirect], |stack| stack
                .unshare_display(display_id));
        }

        // Get stuff to share
        let to_share: Vec<NativeId> = displays
            .into_iter()
            .filter(|native| {
                !self
                    .shared_displays
                    .iter()
                    .any(|(_, native_id)| native_id == native)
            })
            .collect::<Vec<_>>();

        // Get meta info
        let windows = self.native.windows()?;
        let monitors = self.native.monitors()?;

        // Share them, skip errors
        for native_id in to_share {
            let name = match match native_id {
                NativeId::Monitor(m) => monitors
                    .iter()
                    .find(|m1| m1.id == m)
                    .map(|m| m.name.clone()),
                NativeId::Window(w) => windows.iter().find(|w1| w1.id == w).map(|w| w.name.clone()),
            } {
                None => continue, // TODO If we can't find it then just skip I guess
                Some(n) => n,
            };
            let display_id =
                match forward!(self.sv_handler, [HostSignal, HostDirect], |stack| stack
                    .share_display(
                        name,
                        if controllable {
                            AccessMask::CONTROLLABLE
                        } else {
                            AccessMask::empty()
                        }
                    )) {
                    Err(_) => continue, // TODO
                    Ok(display_id) => display_id,
                };
            self.shared_displays.insert(display_id, native_id);
        }

        promise.settle_with(&self.channel, move |mut cx| Ok(cx.undefined()));
        Ok(())
    }

    pub(crate) fn next_auth_scheme(&mut self) -> Result<(), ()> {
        for scheme in [
            AuthSchemeType::None,
            AuthSchemeType::SrpDynamic,
            AuthSchemeType::SrpStatic,
        ] {
            if self.auth_schemes.contains(&scheme) {
                self.auth_schemes.retain(|&x| x != scheme);
                forward!(self.sv_handler, [ClientSignal, ClientDirect], |stack| stack
                    .try_auth(scheme));
                return Ok(());
            }
        }
        Err(())
    }
}

impl Finalize for Instance {}
