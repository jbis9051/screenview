use crate::ChanneledMessage;

use super::DEFAULT_UNRELIABLE_MESSAGE_SIZE;
use common::{messages::Error, sync::event_loop::ThreadWaker};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
    io,
    net::ToSocketAddrs,
};

pub trait Reliable: Sized {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
        waker: ThreadWaker,
    ) -> Result<Self, io::Error>;

    fn send(&mut self, message: Vec<u8>) -> Result<(), SendError>;

    fn close(&mut self);
}

pub trait Unreliable: Sized {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
        waker: ThreadWaker,
    ) -> Result<Self, io::Error>;

    fn send(&mut self, message: Vec<u8>, max_len: usize) -> Result<(), SendError>;

    fn close(&mut self);
}

impl<R: Reliable> Unreliable for R {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
        waker: ThreadWaker,
    ) -> Result<Self, io::Error> {
        <R as Reliable>::new(addr, result_sender, waker)
    }

    fn send(&mut self, message: Vec<u8>, _max_len: usize) -> Result<(), SendError> {
        <R as Reliable>::send(self, message)
    }

    fn close(&mut self) {
        <R as Reliable>::close(self)
    }
}

#[derive(Debug)]
pub struct SendError(pub Source);

impl Display for SendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "send error in {:?}", self.0)
    }
}

impl StdError for SendError {}

pub struct IoHandle<R, U> {
    reliable: Option<R>,
    unreliable: Option<U>,
    receiver: Receiver<TransportResult>,
    result_sender: Sender<TransportResult>,
    unreliable_message_size: usize,
}

impl<R, U> Default for IoHandle<R, U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, U> IoHandle<R, U> {
    /// Constructs a new `IoHandle`.
    ///
    /// The reliable and unreliable portions of this handler are not setup by default, and the
    /// unreliable message size defaults to
    /// [`DEFAULT_UNRELIABLE_MESSAGE_SIZE`](crate::io::DEFAULT_UNRELIABLE_MESSAGE_SIZE).
    pub fn new() -> Self {
        let (result_sender, receiver) = unbounded();

        Self {
            reliable: None,
            unreliable: None,
            result_sender,
            receiver,
            unreliable_message_size: DEFAULT_UNRELIABLE_MESSAGE_SIZE,
        }
    }

    /// Tries to receive a [`TransportResult`] from either the reliable channel or unreliiable
    /// socket according to the method provided.
    ///
    /// [`TransportResult`]: crate::io::TransportResult
    pub fn recv(&self) -> Option<TransportResult> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => unreachable!(),
        }
    }

    /// Returns whether or not the reliable channel is connected.
    pub fn is_reliable_connected(&self) -> bool {
        self.reliable.is_some()
    }

    /// Returns the current maximum unreliable message size. This is the strictly enforced limit on the
    /// maximum message size (in bytes) which can be sent over the unreliable connection. Any message
    /// which goes over this limit will be rejected.
    pub fn max_unreliable_message_size(&self) -> usize {
        self.unreliable_message_size
    }

    /// Sets the maximum unreliable message size to the given value. See [`max_unreliable_message_size`] for
    /// details.
    ///
    /// [`max_unreliable_message_size`](crate::io::IoHandle::max_unreliable_message_size)
    pub fn set_max_unreliable_message_size(&mut self, new_size: usize) {
        self.unreliable_message_size = new_size;
    }
}

impl<R: Reliable, U> IoHandle<R, U> {
    /// Establishes a reliable connection with the given remote address, terminating any existing
    /// connection.
    ///
    /// # Errors
    ///
    /// This function will return an error if the channel it constructs fails to bind to the
    /// given address.
    pub fn connect_reliable<A: ToSocketAddrs>(
        &mut self,
        addr: A,
        waker: ThreadWaker,
    ) -> Result<(), io::Error> {
        self.disconnect_reliable();
        let handle = R::new(addr, self.result_sender.clone(), waker)?;
        self.reliable = Some(handle);
        Ok(())
    }

    /// Terminates any existing connection and attempts to replace that connection using the given
    /// closure.
    pub fn connect_reliable_with<F>(&mut self, f: F)
    where F: FnOnce(Sender<TransportResult>) -> R {
        self.disconnect_reliable();
        let handle = f(self.result_sender.clone());
        self.reliable = Some(handle);
    }

    /// Sends a message through the reliable channel, returning true if it was successfully sent to
    /// the worker thread, or false otherwise.
    ///
    /// # Panics
    ///
    /// Panics if called before a successful call to [`connect_reliable`] is made.
    ///
    /// [`connect_reliable`](crate::io::IoHandle::connect_reliable)
    pub fn send_reliable(&mut self, message: Vec<u8>) -> Result<(), SendError> {
        self.reliable
            .as_mut()
            .expect("reliable connection not established")
            .send(message)
    }

    /// Terminates any existing reliable connection and joins any associated threads.
    pub fn disconnect_reliable(&mut self) {
        if let Some(mut handle) = self.reliable.take() {
            handle.close()
        }
    }
}

impl<R, U: Unreliable> IoHandle<R, U> {
    /// Establishes an unreliable communication channel with the given remote address by binding to
    /// the given socket address.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to bind to the given remote address.
    pub fn connect_unreliable<A: ToSocketAddrs>(
        &mut self,
        addr: A,
        waker: ThreadWaker,
    ) -> Result<(), io::Error> {
        self.disconnect_unreliable();
        let handle = U::new(addr, self.result_sender.clone(), waker)?;
        self.unreliable = Some(handle);
        Ok(())
    }

    /// Sends a message through the unreliable channel, returning true if it was successfully sent
    /// to the worker thread, or false otherwise.
    ///
    /// # Panics
    ///
    /// Panics if called before a successful call to [`connect_unreliable`] is made. If the
    /// given `message`, once serialized, is larger than the [`max_unreliable_message_size`], then it will
    /// cause the worker thread writing to the unreliable channel to reject the message.
    ///
    /// [`connect_unreliable`](crate::io::IoHandle::connect_unreliable)
    /// [`max_unreliable_message_size`](crate::io::IoHandle::max_unreliable_message_size)
    pub fn send_unreliable(&mut self, message: Vec<u8>) -> Result<(), SendError> {
        self.unreliable
            .as_mut()
            .expect("unreliable connection not established")
            .send(message, self.unreliable_message_size)
    }

    /// Unbinds any existing unreliable connection and joins any associated threads.
    pub fn disconnect_unreliable(&mut self) {
        if let Some(mut handle) = self.unreliable.take() {
            handle.close()
        }
    }
}

impl<R: Reliable, U: Unreliable> IoHandle<R, U> {
    pub fn send(&mut self, message: ChanneledMessage<Vec<u8>>) -> Result<(), SendError> {
        match message {
            ChanneledMessage::Reliable(message) => self.send_reliable(message),
            ChanneledMessage::Unreliable(message) => self.send_unreliable(message),
        }
    }
}

pub enum TransportResponse {
    Message(Vec<u8>),
    Shutdown(Source),
}

#[derive(Debug)]
pub enum TransportError {
    TooLarge(usize),
    Recoverable { source: Source, error: Error },
    Fatal { source: Source, error: io::Error },
}

impl Display for TransportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge(size) => write!(
                f,
                "attempted to send too large of a message ({} bytes) through unreliable channel",
                size
            ),
            Self::Recoverable { source, error } =>
                write!(f, "recoverable error in {:?}: {}", source, error),
            Self::Fatal { source, error } => write!(f, "fatal error in {:?}: {}", source, error),
        }
    }
}

impl StdError for TransportError {}

pub type TransportResult = Result<TransportResponse, TransportError>;

#[derive(Debug)]
pub enum Source {
    ReadReliable,
    WriteReliable,
    ReadUnreliable,
    WriteUnreliable,
}
