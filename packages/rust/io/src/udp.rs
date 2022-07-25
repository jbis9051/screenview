use super::{SendError, Source, TransportResult, Unreliable, UDP_READ_SIZE, UDP_TIMEOUT};
use crate::{
    parse_length_field,
    return_if_err,
    TransportError,
    TransportResponse,
    UnreliableState,
    LENGTH_FIELD_WIDTH,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use event_loop::{event_loop::ThreadWaker, JoinOnDrop};
use std::{
    io::{self, ErrorKind},
    net::{ToSocketAddrs, UdpSocket},
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

const BOUND: u8 = 0;
const CONNECTED: u8 = 1;
const SHUTDOWN: u8 = 2;

pub struct UdpHandle {
    write: Sender<(Vec<u8>, usize)>,
    socket: Arc<UdpSocket>,
    state: Arc<AtomicU8>,
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
        let state = Arc::new(AtomicU8::new(BOUND));

        let read_handle = thread::spawn({
            let socket = Arc::clone(&socket);
            let result_sender = result_sender.clone();
            let state = Arc::clone(&state);
            let waker = waker.clone();
            move || {
                read_unreliable(socket, result_sender, &*state, &waker);
                waker.wake();
            }
        });

        let write_handle = thread::spawn({
            let socket = Arc::clone(&socket);
            move || {
                write_unreliable(socket, result_sender, write_rx, &waker);
                waker.wake();
            }
        });

        Ok(Self {
            write: write_tx,
            socket,
            state,
            _handles: Box::new((JoinOnDrop::new(read_handle), JoinOnDrop::new(write_handle))),
        })
    }

    fn connect<A: ToSocketAddrs>(&self, addr: A) -> Result<(), io::Error> {
        let res = self.socket.connect(addr);
        if res.is_ok() {
            self.state.store(CONNECTED, Ordering::Relaxed);
        }
        res
    }

    fn state(&self) -> UnreliableState {
        match self.state.load(Ordering::Relaxed) {
            BOUND => UnreliableState::Bound,
            CONNECTED => UnreliableState::Connected,
            _ => unreachable!(),
        }
    }

    fn send(&mut self, message: Vec<u8>, max_len: usize) -> Result<(), SendError> {
        self.write
            .send((message, max_len))
            .map_err(|_| SendError(Source::WriteUnreliable))
    }

    fn close(&mut self) {
        self.state.store(SHUTDOWN, Ordering::Relaxed);
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
    shutdown: &AtomicU8,
    waker: &ThreadWaker,
) {
    let mut buffer = vec![0u8; UDP_READ_SIZE];

    while shutdown.load(Ordering::Relaxed) != SHUTDOWN {
        let (read, addr) = match socket.recv_from(&mut buffer[..]) {
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

        return_if_err!(sender.send(Ok(TransportResponse::UnreliableMessage(
            buffer[LENGTH_FIELD_WIDTH .. read].to_vec(),
            addr
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
        if message.len() > max_len {
            return_if_err!(sender.send(Err(TransportError::TooLarge(message.len(), max_len))));
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
