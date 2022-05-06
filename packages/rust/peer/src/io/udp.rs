use super::{Source, TransportResult, Unreliable, UDP_READ_SIZE, UDP_TIMEOUT};
use crate::{
    io::{parse_length_field, TransportError, TransportResponse, LENGTH_FIELD_WIDTH},
    return_if_err,
};
use common::event_loop::{JoinOnDrop, ThreadWaker};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    io::{self, ErrorKind},
    net::{ToSocketAddrs, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub struct UdpHandle {
    write: Sender<(Vec<u8>, usize)>,
    shutdown: Arc<AtomicBool>,
    _handles: Box<(JoinOnDrop<()>, JoinOnDrop<()>)>,
}

impl Unreliable for UdpHandle {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
        waker: ThreadWaker,
    ) -> Result<Self, io::Error> {
        let socket = Arc::new(UdpSocket::bind(addr)?);
        socket.set_read_timeout(Some(Duration::from_millis(UDP_TIMEOUT)))?;
        let (write_tx, write_rx) = unbounded();
        let shutdown = Arc::new(AtomicBool::new(true));

        let read_handle = thread::spawn({
            let socket = Arc::clone(&socket);
            let result_sender = result_sender.clone();
            let shutdown = Arc::clone(&shutdown);
            let waker = waker.clone();
            move || {
                read_unreliable(socket, result_sender, &*shutdown, &waker);
                waker.wake();
            }
        });

        let write_handle = thread::spawn(move || {
            write_unreliable(socket, result_sender, write_rx, &waker);
            waker.wake();
        });

        Ok(Self {
            write: write_tx,
            shutdown,
            _handles: Box::new((JoinOnDrop::new(read_handle), JoinOnDrop::new(write_handle))),
        })
    }

    fn send(&mut self, message: Vec<u8>, max_len: usize) -> Result<(), ()> {
        self.write.send((message, max_len)).map_err(|_| ())
    }

    fn close(&mut self) {
        self.shutdown.store(false, Ordering::Relaxed);
    }
}

impl Drop for UdpHandle {
    fn drop(&mut self) {
        self.close();
    }
}

fn read_unreliable(
    socket: Arc<UdpSocket>,
    sender: Sender<TransportResult>,
    shutdown: &AtomicBool,
    waker: &ThreadWaker,
) {
    let mut buffer = vec![0u8; UDP_READ_SIZE];

    while shutdown.load(Ordering::Relaxed) {
        let read = match socket.recv(&mut buffer[..]) {
            Ok(read) => read,
            Err(error)
                if error.kind() == ErrorKind::WouldBlock || error.kind() == ErrorKind::TimedOut =>
                continue,
            Err(error) => {
                let _ = sender.send(Err(TransportError::Fatal {
                    source: Source::ReadUnreliable,
                    error,
                }));
                return;
            }
        };

        if read == 0 {
            let _ = sender.send(Ok(TransportResponse::Shutdown(Source::ReadUnreliable)));
            return;
        }

        if read < LENGTH_FIELD_WIDTH {
            // Drop a packet which is an invalid length
            continue;
        }

        let received_length = parse_length_field(&buffer);
        if read - LENGTH_FIELD_WIDTH != received_length {
            // Drop a packet if the lengths don't match
            continue;
        }

        return_if_err!(sender.send(Ok(TransportResponse::Message(
            buffer[LENGTH_FIELD_WIDTH .. read].to_vec()
        ))));
        waker.wake();
    }
}

fn write_unreliable(
    socket: Arc<UdpSocket>,
    sender: Sender<TransportResult>,
    receiver: Receiver<(Vec<u8>, usize)>,
    waker: &ThreadWaker,
) {
    while let Ok((message, max_len)) = receiver.recv() {
        if message.len() + LENGTH_FIELD_WIDTH > max_len {
            return_if_err!(sender.send(Err(TransportError::TooLarge(message.len()))));
            waker.wake();
            continue;
        }

        if let Err(error) = socket.send(&*message) {
            let _ = sender.send(Err(TransportError::Fatal {
                source: Source::WriteUnreliable,
                error,
            }));
            return;
        }

        waker.wake();
    }
}
