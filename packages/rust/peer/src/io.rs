use common::messages::{sel::*, Error, MessageComponent, MessageID};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use futures_executor::block_on;
use futures_util::{future::FutureExt, pin_mut, select_biased};
use std::{
    io,
    io::{Cursor, Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    ptr,
    sync::Arc,
    thread,
    thread::JoinHandle,
};
use tokio::{net::UdpSocket, sync::oneshot};

const INIT_BUFFER_CAPACITY: usize = 4096;
const MAX_SERVER_HELLO_LEN: usize = 0x04_00_00_00;
const UDP_READ_SIZE: usize = 65507;
pub const DEFAULT_UDP_MESSAGE_SIZE: usize = 1500;

macro_rules! return_if_err {
    ($expr:expr) => {
        if let Err(_) = $expr {
            return;
        }
    };
}

pub struct IoHandle {
    reliable: Option<ReliableHandle>,
    unreliable: Option<UnreliableHandle>,
    receiver: Receiver<TransportResult>,
    result_sender: Sender<TransportResult>,
    udp_message_size: usize,
}

impl IoHandle {
    /// Constructs a new `IoHandle`.
    ///
    /// The reliable and unreliable portions of this handler are not setup by default, and the
    /// UDP message size defaults to
    /// [`DEFAULT_UDP_MESSAGE_SIZE`](crate::io::DEFAULT_UDP_MESSAGE_SIZE).
    pub fn new() -> Self {
        let (result_sender, receiver) = unbounded();

        Self {
            reliable: None,
            unreliable: None,
            result_sender,
            receiver,
            udp_message_size: DEFAULT_UDP_MESSAGE_SIZE,
        }
    }

    /// Tries to receive a [`TransportResult`] from either the reliable channel or unreliiable
    /// socket without blocking, returning it if it exists.
    ///
    /// [`TransportResult`]: crate::io::TransportResult
    pub fn recv(&self) -> Option<TransportResult> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => unreachable!(),
        }
    }

    /// Establishes a reliable connection with the given remote address, terminating any existing
    /// connection.
    ///
    /// # Errors
    ///
    /// This function will return an error if the TCP stream it constructs fails to bind to the
    /// given address.
    pub fn connect_reliable<A: ToSocketAddrs>(&mut self, addr: A) -> Result<(), io::Error> {
        drop(self.reliable.take());

        let handle = ReliableHandle::new(addr, self.result_sender.clone())?;
        self.reliable = Some(handle);
        Ok(())
    }

    /// Sends a message through the reliable channel, returning true if it was successfully sent to
    /// the worker thread, or false otherwise.
    ///
    /// # Panics
    ///
    /// Panics if called before a successful call to [`connect_reliable`] is made.
    ///
    /// [`connect_reliable`](crate::io::IoHandle::connect_reliable)
    pub fn send_reliable(&self, message: SelMessage) -> bool {
        self.reliable
            .as_ref()
            .expect("reliable connection not established")
            .send(message)
    }

    /// Terminates any existing TCP connection and joins any associated threads.
    pub fn disconnect_reliable(&mut self) {
        self.reliable = None;
    }

    /// Establishes an unreliable communication channel with the given remote address by binding to
    /// a UDP socket.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to bind a UDP socket to the given remote
    /// address. It will also return an error if it fails to convert the std UDP socket into an
    /// async socket.
    pub fn connect_unreliable<A: ToSocketAddrs>(&mut self, addr: A) -> Result<(), io::Error> {
        drop(self.unreliable.take());

        let handle = UnreliableHandle::new(addr, self.result_sender.clone())?;
        self.unreliable = Some(handle);
        Ok(())
    }

    /// Sends a message through the unreliable channel, returning true if it was successfully sent
    /// to the worker thread, or false otherwise.
    ///
    /// # Panics
    ///
    /// Panics if called before a successful call to [`connect_unreliable`] is made. If the
    /// given `message`, once serialized, is larger than the [`max_udp_message_size`], then it will
    /// cause the worker thread writing to the UDP socket to reject the message.
    ///
    /// [`connect_unreliable`](crate::io::IoHandle::connect_unreliable)
    /// [`max_udp_message_size`](crate::io::IoHandle::max_udp_message_size)
    pub fn send_unreliable(&self, message: SelMessage) -> bool {
        self.unreliable
            .as_ref()
            .expect("unreliable connection not established")
            .send(message, self.udp_message_size)
    }

    /// Returns the current maximum UDP message size. This is the strictly enforced limit on the
    /// maximum message size (in bytes) which can be sent over UDP. Any message which goes over
    /// this limit will be rejected.
    pub fn max_udp_message_size(&self) -> usize {
        self.udp_message_size
    }

    /// Sets the maximum UDP message size to the given value. See [`max_udp_message_size`] for
    /// details.
    ///
    /// [`max_udp_message_size`](crate::io::IoHandle::max_udp_message_size)
    pub fn set_max_udp_message_size(&mut self, new_size: usize) {
        self.udp_message_size = new_size;
    }

    /// Unbinds any existing UDP socket and joins any associated threads.
    pub fn disconnect_unreliable(&mut self) {
        self.unreliable = None;
    }
}

struct ReliableHandle {
    stream: Arc<TcpStream>,
    write: Sender<SelMessage>,
    _handles: (JoinOnDrop<()>, JoinOnDrop<()>),
}

impl ReliableHandle {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
    ) -> Result<Self, io::Error> {
        let stream = Arc::new(TcpStream::connect(addr)?);
        let (write_tx, write_rx) = unbounded();

        let read_handle = thread::spawn({
            let stream = Arc::clone(&stream);
            let result_sender = result_sender.clone();
            move || read_reliable(stream, result_sender)
        });

        let write_handle = thread::spawn({
            let stream = Arc::clone(&stream);
            let result_sender = result_sender.clone();
            move || write_reliable(stream, result_sender, write_rx)
        });

        Ok(Self {
            stream,
            write: write_tx,
            _handles: (JoinOnDrop::new(read_handle), JoinOnDrop::new(write_handle)),
        })
    }

    fn send(&self, message: SelMessage) -> bool {
        self.write.send(message).is_ok()
    }
}

impl Drop for ReliableHandle {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

struct UnreliableHandle {
    write: Sender<(SelMessage, usize)>,
    shutdown: Option<oneshot::Sender<()>>,
    _handles: (JoinOnDrop<()>, JoinOnDrop<()>),
}

impl UnreliableHandle {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
    ) -> Result<Self, io::Error> {
        let socket = std::net::UdpSocket::bind(addr)?;
        let socket = Arc::new(UdpSocket::from_std(socket)?);
        let (write_tx, write_rx) = unbounded();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let read_handle = thread::spawn({
            let socket = Arc::clone(&socket);
            let result_sender = result_sender.clone();
            move || block_on(read_unreliable(socket, result_sender, shutdown_rx))
        });

        let write_handle = thread::spawn({
            let result_sender = result_sender.clone();
            move || block_on(write_unreliable(socket, result_sender, write_rx))
        });

        Ok(Self {
            write: write_tx,
            shutdown: Some(shutdown_tx),
            _handles: (JoinOnDrop::new(read_handle), JoinOnDrop::new(write_handle)),
        })
    }

    fn send(&self, message: SelMessage, max_len: usize) -> bool {
        self.write.send((message, max_len)).is_ok()
    }
}

impl Drop for UnreliableHandle {
    fn drop(&mut self) {
        let _ = self.shutdown.take().unwrap().send(());
    }
}

struct JoinOnDrop<T> {
    handle: Option<JoinHandle<T>>,
}

impl<T> JoinOnDrop<T> {
    fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl<T> Drop for JoinOnDrop<T> {
    fn drop(&mut self) {
        // Drop can only ever be called once so unwrap will never fail
        drop(self.handle.take().unwrap().join());
    }
}

pub enum TransportResult {
    Ok(SelMessage),
    TooLarge(SelMessage),
    Recoverable { source: Source, error: Error },
    Fatal { source: Source, error: io::Error },
    Shutdown(Source),
}

pub enum Source {
    ReadReliable,
    WriteReliable,
    ReadUnreliable,
    WriteUnreliable,
}

fn read_reliable(stream: Arc<TcpStream>, sender: Sender<TransportResult>) {
    // While reading:
    // - If no data is present, wait for more data
    // - If data is present, collect the message and parse it, otherwise return

    let mut buffer = vec![0u8; INIT_BUFFER_CAPACITY];
    let mut data_end = 0usize;

    loop {
        // No more data is left in the buffer, so we wait for more
        if data_end == 0 {
            data_end = match Read::read(&mut (&*stream), &mut buffer[..]) {
                Ok(data_end) => data_end,
                Err(error) => {
                    let _ = sender.send(TransportResult::Fatal {
                        source: Source::ReadReliable,
                        error,
                    });
                    return;
                }
            };

            // The syscall exited successfully, but no data remained, meaning the stream closed
            if data_end == 0 {
                let _ = sender.send(TransportResult::Shutdown(Source::ReadReliable));
                return;
            }
        }

        // Collect and parse the message
        let data_parsed = match collect_and_parse_reliable(&*stream, &mut buffer, &mut data_end) {
            Ok((message, data_parsed)) => {
                return_if_err!(sender.send(TransportResult::Ok(message)));

                data_parsed
            }
            Err(error) => {
                let res = sender.send(TransportResult::Recoverable {
                    source: Source::ReadReliable,
                    error,
                });
                return_if_err!(res);

                // If we hit a recoverable, we disregard the rest of the bytes in the buffer
                data_end
            }
        };

        // If there are more messages in the buffer, move their data to the beginning of the buffer
        if data_parsed < data_end {
            unsafe {
                let src = buffer.as_ptr().add(data_parsed);
                let dst = buffer.as_mut_ptr();
                ptr::copy(src, dst, data_end - data_parsed);
            }
        } else {
            debug_assert!(data_parsed == data_end);
        }

        data_end -= data_parsed;
    }
}

fn collect_and_parse_reliable(
    mut stream: &TcpStream,
    buffer: &mut Vec<u8>,
    data_end: &mut usize,
) -> Result<(SelMessage, usize), Error> {
    let id = buffer[0];

    // Collect the remaining bytes if necessary, and parse the message'
    match id {
        TransportDataMessageReliable::ID => {
            // Similar to ServerHello, if the length gets cut off, then we read the rest of it here
            if *data_end < 3 {
                Read::read_exact(&mut stream, &mut buffer[*data_end .. 3])?;
            }

            let mut length_bytes = [0u8; 2];
            length_bytes.copy_from_slice(&buffer[1 .. 3]);
            // Add 3 to account for ID byte and length bytes
            let length = match usize::from(u16::from_le_bytes(length_bytes)).checked_add(3) {
                Some(len) => len,
                None => return Err(Error::BadSelMessage),
            };

            collect_reliable(stream, buffer, data_end, length)?;
            let message = TransportDataMessageReliable::read(&mut Cursor::new(&buffer[3 ..]))?;
            Ok((SelMessage::TransportDataMessageReliable(message), length))
        }
        _ => Err(Error::BadMessageID(id)),
    }
}

#[inline]
fn collect_reliable(
    mut stream: &TcpStream,
    buffer: &mut Vec<u8>,
    data_end: &mut usize,
    length: usize,
) -> io::Result<()> {
    if length > *data_end {
        if length > buffer.len() {
            buffer.resize(length, 0u8);
        }

        Read::read_exact(&mut stream, &mut buffer[*data_end .. length])?;
        *data_end = length;
    }

    Ok(())
}

fn write_reliable(
    stream: Arc<TcpStream>,
    sender: Sender<TransportResult>,
    receiver: Receiver<SelMessage>,
) {
    let mut buffer = Vec::with_capacity(INIT_BUFFER_CAPACITY);

    while let Ok(ref msg) = receiver.recv() {
        let mut cursor = Cursor::new(buffer);

        let res = MessageComponent::write(msg, &mut cursor);
        buffer = cursor.into_inner();

        match res {
            Ok(_) =>
                if let Err(error) = (&*stream).write_all(&buffer) {
                    let _ = sender.send(TransportResult::Fatal {
                        source: Source::WriteReliable,
                        error,
                    });
                    return;
                },
            Err(error) => {
                let res = sender.send(TransportResult::Recoverable {
                    source: Source::WriteReliable,
                    error,
                });
                return_if_err!(res);
            }
        }

        buffer.clear();
    }
}

async fn read_unreliable(
    socket: Arc<UdpSocket>,
    sender: Sender<TransportResult>,
    shutdown: oneshot::Receiver<()>,
) {
    let mut buffer = vec![0u8; UDP_READ_SIZE];
    let shutdown = shutdown.fuse();
    pin_mut!(shutdown);

    loop {
        let read = {
            let read_fut = socket.recv(&mut buffer[..]).fuse();
            pin_mut!(read_fut);

            select_biased! {
                res = read_fut => match res {
                    Ok(read) => read,
                    Err(error) => {
                        let _ = sender.send(TransportResult::Fatal {
                            source: Source::ReadUnreliable,
                            error,
                        });
                        return;
                    }
                },
                _ = shutdown => return
            }
        };

        if read == 0 {
            let _ = sender.send(TransportResult::Shutdown(Source::ReadUnreliable));
            return;
        }

        let mut cursor = Cursor::new(&buffer[.. read]);

        let transport_result = match SelMessage::read(&mut cursor) {
            Ok(message) => TransportResult::Ok(message),
            Err(error) => TransportResult::Recoverable {
                source: Source::ReadUnreliable,
                error,
            },
        };

        return_if_err!(sender.send(transport_result));
    }
}

async fn write_unreliable(
    socket: Arc<UdpSocket>,
    sender: Sender<TransportResult>,
    receiver: Receiver<(SelMessage, usize)>,
) {
    let mut buffer = Vec::with_capacity(1500);

    while let Ok((message, max_len)) = receiver.recv() {
        let mut cursor = Cursor::new(buffer);

        let res = MessageComponent::write(&message, &mut cursor);
        buffer = cursor.into_inner();

        if buffer.len() > max_len {
            return_if_err!(sender.send(TransportResult::TooLarge(message)));
        }

        match res {
            Ok(_) =>
                if let Err(error) = socket.send(&buffer[..]).await {
                    let _ = sender.send(TransportResult::Fatal {
                        source: Source::WriteUnreliable,
                        error,
                    });
                    return;
                },
            Err(error) => {
                let res = sender.send(TransportResult::Recoverable {
                    source: Source::WriteUnreliable,
                    error,
                });
                return_if_err!(res);
            }
        }

        buffer.clear();
    }
}
