use super::{JoinOnDrop, Source, TransportResult, Unreliable, UDP_READ_SIZE};
use crate::{
    io::{TransportError, TransportResponse},
    return_if_err,
};
use common::{
    event_loop::ThreadWaker,
    messages::{sel::*, MessageComponent},
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use futures_executor::block_on;
use futures_util::{future::FutureExt, pin_mut, select_biased};
use std::{io, io::Cursor, net::ToSocketAddrs, sync::Arc, thread};
use tokio::{net::UdpSocket, sync::oneshot};

pub struct UdpHandle {
    write: Sender<(SelMessage, usize)>,
    shutdown: Option<oneshot::Sender<()>>,
    _handles: Box<(JoinOnDrop<()>, JoinOnDrop<()>)>,
}

impl Unreliable for UdpHandle {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
        waker: ThreadWaker,
    ) -> Result<Self, io::Error> {
        let socket = std::net::UdpSocket::bind(addr)?;
        let socket = Arc::new(UdpSocket::from_std(socket)?);
        let (write_tx, write_rx) = unbounded();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let read_handle = thread::spawn({
            let socket = Arc::clone(&socket);
            let result_sender = result_sender.clone();
            let waker = waker.clone();
            move || {
                block_on(read_unreliable(socket, result_sender, shutdown_rx, &waker));
                waker.wake();
            }
        });

        let write_handle = thread::spawn(move || {
            block_on(write_unreliable(socket, result_sender, write_rx, &waker));
            waker.wake();
        });

        Ok(Self {
            write: write_tx,
            shutdown: Some(shutdown_tx),
            _handles: Box::new((JoinOnDrop::new(read_handle), JoinOnDrop::new(write_handle))),
        })
    }

    fn send(&mut self, message: SelMessage, max_len: usize) -> bool {
        self.write.send((message, max_len)).is_ok()
    }

    fn close(&mut self) {
        let _ = self.shutdown.take().map(|sender| sender.send(()));
    }
}

impl Drop for UdpHandle {
    fn drop(&mut self) {
        self.close();
    }
}

async fn read_unreliable(
    socket: Arc<UdpSocket>,
    sender: Sender<TransportResult>,
    shutdown: oneshot::Receiver<()>,
    waker: &ThreadWaker,
) {
    let mut buffer = vec![0u8; UDP_READ_SIZE];
    let shutdown = shutdown.fuse();
    pin_mut!(shutdown);

    loop {
        let read = {
            let read_fut = socket.recv(&mut buffer[..]).fuse();
            pin_mut!(read_fut);

            select_biased! {
                _ = shutdown => return,
                res = read_fut => match res {
                    Ok(read) => read,
                    Err(error) => {
                        let _ = sender.send(Err(TransportError::Fatal {
                            source: Source::ReadUnreliable,
                            error,
                        }));
                        return;
                    }
                },
            }
        };

        if read == 0 {
            let _ = sender.send(Ok(TransportResponse::Shutdown(Source::ReadUnreliable)));
            return;
        }

        let mut cursor = Cursor::new(&buffer[.. read]);

        let transport_result = match SelMessage::read(&mut cursor) {
            Ok(message) => Ok(TransportResponse::Message(message)),
            Err(error) => Err(TransportError::Recoverable {
                source: Source::ReadUnreliable,
                error,
            }),
        };

        return_if_err!(sender.send(transport_result));
        waker.wake();
    }
}

async fn write_unreliable(
    socket: Arc<UdpSocket>,
    sender: Sender<TransportResult>,
    receiver: Receiver<(SelMessage, usize)>,
    waker: &ThreadWaker,
) {
    let mut buffer = Vec::with_capacity(1500);

    while let Ok((message, max_len)) = receiver.recv() {
        let mut cursor = Cursor::new(buffer);

        let res = MessageComponent::write(&message, &mut cursor);
        buffer = cursor.into_inner();

        if buffer.len() > max_len {
            return_if_err!(sender.send(Err(TransportError::TooLarge(message))));
            waker.wake();
            continue;
        }

        match res {
            Ok(_) =>
                if let Err(error) = socket.send(&buffer[..]).await {
                    let _ = sender.send(Err(TransportError::Fatal {
                        source: Source::WriteUnreliable,
                        error,
                    }));
                    return;
                },
            Err(error) => {
                let res = sender.send(Err(TransportError::Recoverable {
                    source: Source::WriteUnreliable,
                    error,
                }));
                return_if_err!(res);
                waker.wake();
            }
        }

        buffer.clear();
    }
}
