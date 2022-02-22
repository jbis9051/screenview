use super::DEFAULT_UNRELIABLE_MESSAGE_SIZE;
use common::messages::{sel::*, Error};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use std::{io, net::ToSocketAddrs};

pub trait Reliable: Sized {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
    ) -> Result<Self, io::Error>;

    fn send(&mut self, message: SelMessage) -> bool;

    fn close(&mut self);
}

pub trait Unreliable: Sized {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
    ) -> Result<Self, io::Error>;

    fn send(&mut self, message: SelMessage, max_len: usize) -> bool;

    fn close(&mut self);
}

pub struct IoHandle<R, U> {
    reliable: Option<R>,
    unreliable: Option<U>,
    receiver: Receiver<TransportResult>,
    result_sender: Sender<TransportResult>,
    unreliable_message_size: usize,
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
    pub fn connect_reliable<A: ToSocketAddrs>(&mut self, addr: A) -> Result<(), io::Error> {
        self.disconnect_reliable();
        let handle = R::new(addr, self.result_sender.clone())?;
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
    pub fn send_reliable(&mut self, message: SelMessage) -> bool {
        self.reliable
            .as_mut()
            .expect("reliable connection not established")
            .send(message)
    }

    /// Terminates any existing reliable connection and joins any associated threads.
    pub fn disconnect_reliable(&mut self) {
        self.reliable.take().map(|mut handle| handle.close());
    }
}

impl<R, U: Unreliable> IoHandle<R, U> {
    /// Establishes an unreliable communication channel with the given remote address by binding to
    /// the given socket address.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to bind to the given remote address.
    pub fn connect_unreliable<A: ToSocketAddrs>(&mut self, addr: A) -> Result<(), io::Error> {
        self.disconnect_unreliable();
        let handle = U::new(addr, self.result_sender.clone())?;
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
    pub fn send_unreliable(&mut self, message: SelMessage) -> bool {
        self.unreliable
            .as_mut()
            .expect("unreliable connection not established")
            .send(message, self.unreliable_message_size)
    }

    /// Unbinds any existing unreliable connection and joins any associated threads.
    pub fn disconnect_unreliable(&mut self) {
        self.unreliable.take().map(|mut handle| handle.close());
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