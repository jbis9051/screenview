use super::{
    parse_length_field,
    Reliable,
    SendError,
    Source,
    TransportError,
    TransportResponse,
    TransportResult,
    INIT_BUFFER_CAPACITY,
    LENGTH_FIELD_WIDTH,
};
use crate::return_if_err;
use common::messages::Error;
use crossbeam_channel::{unbounded, Receiver, Sender};
use event_loop::{event_loop::ThreadWaker, JoinOnDrop};
use std::{
    io,
    io::{Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    sync::Arc,
    thread,
};

pub struct TcpHandle {
    stream: Arc<TcpStream>,
    write: Sender<Vec<u8>>,
    _handles: Box<(JoinOnDrop<()>, JoinOnDrop<()>)>,
}

impl TcpHandle {
    pub fn new_from(
        stream: TcpStream,
        result_sender: Sender<TransportResult>,
        waker: ThreadWaker,
    ) -> Self {
        let stream = Arc::new(stream);
        let (write_tx, write_rx) = unbounded();

        let read_handle = thread::spawn({
            let stream = Arc::clone(&stream);
            let result_sender = result_sender.clone();
            let waker = waker.clone();
            move || {
                read_reliable(stream, result_sender, &waker);
                waker.wake();
            }
        });

        let write_handle = thread::spawn({
            let stream = Arc::clone(&stream);
            move || {
                write_reliable(stream, result_sender, write_rx, &waker);
                waker.wake();
            }
        });

        Self {
            stream,
            write: write_tx,
            _handles: Box::new((JoinOnDrop::new(read_handle), JoinOnDrop::new(write_handle))),
        }
    }
}

impl Reliable for TcpHandle {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
        waker: ThreadWaker,
    ) -> Result<Self, io::Error> {
        Ok(Self::new_from(
            TcpStream::connect(addr)?,
            result_sender,
            waker,
        ))
    }

    fn send(&mut self, message: Vec<u8>) -> Result<(), SendError> {
        self.write
            .send(message)
            .map_err(|_| SendError(Source::WriteReliable))
    }

    fn close(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

impl Drop for TcpHandle {
    fn drop(&mut self) {
        self.close();
    }
}


fn read_reliable(stream: Arc<TcpStream>, sender: Sender<TransportResult>, waker: &ThreadWaker) {
    // While reading:
    // - If no data is present, wait for more data
    // - If data is present, collect the message and parse it, otherwise return

    let mut buffer = vec![0u8; INIT_BUFFER_CAPACITY];
    let mut data_end = 0usize; // One byte past the index of the last byte of data in the buffer

    loop {
        // No more data is left in the buffer, so we wait for more
        if data_end == 0 {
            data_end = match Read::read(&mut (&*stream), &mut buffer[..]) {
                Ok(data_end) => data_end,
                Err(error) => {
                    // TODO I don't think this is really an error
                    let _ = sender.send(Err(TransportError::Fatal {
                        source: Source::ReadReliable,
                        error,
                    }));
                    return;
                }
            };

            // The syscall exited successfully, but no data remained, meaning the stream closed
            if data_end == 0 {
                let _ = sender.send(Ok(TransportResponse::Shutdown(Source::ReadReliable)));
                return;
            }
        }

        // Collect and parse the message
        let data_parsed = match collect_and_parse_reliable(&*stream, &mut buffer, &mut data_end) {
            Ok(message) => {
                let message_len = message.len();
                return_if_err!(sender.send(Ok(TransportResponse::ReliableMessage(message))));

                message_len + LENGTH_FIELD_WIDTH
            }
            Err(error) => {
                let res = sender.send(Err(TransportError::Recoverable {
                    source: Source::ReadReliable,
                    error,
                }));
                return_if_err!(res);

                // If we hit a recoverable, we disregard the rest of the bytes in the buffer
                data_end
            }
        };

        // Notify the handler thread that a message is ready
        waker.wake();

        // If there are more messages in the buffer, move their data to the beginning of the buffer
        if data_parsed < data_end {
            buffer.drain(.. data_parsed);
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
) -> Result<Vec<u8>, Error> {
    if *data_end < LENGTH_FIELD_WIDTH {
        // We need to read the length field but there isn't enough data in the buffer to do so
        Read::read_exact(&mut stream, &mut buffer[*data_end .. LENGTH_FIELD_WIDTH])?;
    }

    // The length field indicates the remaining length of the message so we add on the length
    // field width to get the total length
    let length = match parse_length_field(buffer).checked_add(LENGTH_FIELD_WIDTH) {
        Some(len) => len,
        None => return Err(Error::BadTransportMessage),
    };

    collect_reliable(stream, buffer, data_end, length)?;
    Ok(buffer[LENGTH_FIELD_WIDTH .. length].to_vec())
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
    receiver: Receiver<Vec<u8>>,
    waker: &ThreadWaker,
) {
    while let Ok(msg) = receiver.recv() {
        if let Err(error) = (&*stream).write_all(&*msg) {
            let _ = sender.send(Err(TransportError::Fatal {
                source: Source::WriteReliable,
                error,
            }));
            return;
        }

        waker.wake();
    }
}
