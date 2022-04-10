use crate::protocol::{ConnectionType, Message, PromiseHandle, RequestContent};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use native::{NativeApi, NativeApiError};
use neon::prelude::{Context, Finalize, JsUndefined};
use peer::{
    io::{TcpHandle, UdpHandle},
    services::ScreenViewHandler,
};
use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct InstanceHandle {
    sender: Sender<Message>,
    thread_handle: Option<JoinHandle<()>>,
}

impl InstanceHandle {
    fn new(sender: Sender<Message>, thread_handle: JoinHandle<()>) -> Self {
        Self {
            sender,
            thread_handle: Some(thread_handle),
        }
    }

    pub fn send(&self, message: Message) -> bool {
        self.sender.send(message).is_ok()
    }
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Shutdown);
        let _ = self.thread_handle.take().unwrap().join();
    }
}

impl Finalize for InstanceHandle {}

pub struct Instance {
    native: NativeApi,
    sv_handler: ScreenViewHandler<TcpHandle, UdpHandle>,
}

impl Instance {
    pub fn new_host() -> Result<InstanceHandle, NativeApiError> {
        let (tx, rx) = unbounded();
        let instance = Self {
            native: NativeApi::new()?,
            sv_handler: ScreenViewHandler::new_host(),
        };
        let thread_handle = start_instance_main(instance, rx);

        Ok(InstanceHandle::new(tx, thread_handle))
    }

    pub fn new_client() -> Result<InstanceHandle, NativeApiError> {
        let (tx, rx) = unbounded();
        let instance = Self {
            native: NativeApi::new()?,
            sv_handler: ScreenViewHandler::new_client(),
        };
        let thread_handle = start_instance_main(instance, rx);

        Ok(InstanceHandle::new(tx, thread_handle))
    }

    fn handle_node_request(
        &mut self,
        content: RequestContent,
        promise: PromiseHandle,
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
        promise: PromiseHandle,
    ) -> Result<(), anyhow::Error> {
        let io_handle = self.sv_handler.io_handle();

        let result = match connection_type {
            ConnectionType::Reliable => io_handle.connect_reliable(addr),
            ConnectionType::Unreliable => io_handle.connect_unreliable(addr),
        };

        let result = result.map_err(|error| error.to_string());

        promise.deferred.settle_with::<JsUndefined, _>(
            &promise.channel,
            move |mut cx| match result {
                Ok(()) => Ok(cx.undefined()),
                Err(error) => {
                    let error = cx.error(error)?;
                    cx.throw(error)
                }
            },
        );

        Ok(())
    }
}

impl Finalize for Instance {}

fn start_instance_main(instance: Instance, message_receiver: Receiver<Message>) -> JoinHandle<()> {
    thread::spawn(move || instance_main(instance, message_receiver))
}

fn instance_main(mut instance: Instance, message_receiver: Receiver<Message>) {
    const MIN_DELAY: Duration = Duration::from_millis(5);
    const MAX_DELAY: Duration = Duration::from_millis(200);
    const STEP: Duration = Duration::from_millis(5);
    let mut delay = MIN_DELAY;

    loop {
        let mut did_work = false;

        // Handle incoming messages from node
        match message_receiver.try_recv() {
            Ok(Message::Request { content, promise }) => {
                match instance.handle_node_request(content, promise) {
                    Ok(()) => { /* yay */ }
                    Err(error) => panic!("Internal error while handling node request: {}", error),
                }

                did_work = true;
            }
            Ok(Message::Shutdown) => return,
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return,
        }

        // Handle messages from remote party
        match instance.sv_handler.handle_next_message() {
            Some(Ok(events)) => {
                for event in events {
                    todo!("Handle event")
                }

                did_work = true;
            }
            Some(Err(error)) => todo!("Handle this error properly: {}", error),
            None => {}
        }

        if did_work {
            if delay > MIN_DELAY {
                delay -= STEP;
            }
        } else {
            thread::sleep(delay);
            if delay < MAX_DELAY {
                delay += STEP;
            }
        }
    }
}
