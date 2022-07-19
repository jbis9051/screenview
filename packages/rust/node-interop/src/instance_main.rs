use crate::{
    event_handler::handle_event,
    forward,
    instance::Instance,
    protocol::Message,
    screenview_handler::ScreenViewHandler,
};
use crossbeam_channel::{Receiver, TryRecvError};
use event_loop::event_loop::{event_loop, EventLoopState, ThreadWakerCore};
use std::{thread, thread::JoinHandle};

#[repr(u32)]
pub enum Events {
    RemoteMessage,
    InteropMessage,
    DirectServerConnection,
    FrameUpdate,
}

pub fn start_instance_main<F>(
    make_instance: F,
    message_receiver: Receiver<Message>,
) -> JoinHandle<()>
where
    F: FnOnce(&ThreadWakerCore) -> Instance + Send + 'static,
{
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
                        for event in events {
                            handle_event(&mut instance, event);
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
                let mut frame_update = match capture.next_update() {
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
