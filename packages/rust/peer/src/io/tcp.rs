use super::{
    Reliable,
    Source,
    TransportError,
    TransportResponse,
    TransportResult,
    INIT_BUFFER_CAPACITY,
};
use crate::return_if_err;
use common::{
    event_loop::{JoinOnDrop, ThreadWaker},
    messages::{sel::*, Error, MessageComponent, MessageID},
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    io,
    io::{Cursor, Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    ptr,
    sync::Arc,
    thread,
};

pub struct TcpHandle {
    stream: Arc<TcpStream>,
    write: Sender<SelMessage>,
    _handles: Box<(JoinOnDrop<()>, JoinOnDrop<()>)>,
}

impl Reliable for TcpHandle {
    fn new<A: ToSocketAddrs>(
        addr: A,
        result_sender: Sender<TransportResult>,
        waker: ThreadWaker,
    ) -> Result<Self, io::Error> {
        let stream = Arc::new(TcpStream::connect(addr)?);
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

        Ok(Self {
            stream,
            write: write_tx,
            _handles: Box::new((JoinOnDrop::new(read_handle), JoinOnDrop::new(write_handle))),
        })
    }

    fn send(&mut self, message: SelMessage) -> bool {
        self.write.send(message).is_ok()
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
    let mut data_end = 0usize;

    loop {
        // No more data is left in the buffer, so we wait for more
        if data_end == 0 {
            data_end = match Read::read(&mut (&*stream), &mut buffer[..]) {
                Ok(data_end) => data_end,
                Err(error) => {
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
            Ok((message, data_parsed)) => {
                return_if_err!(sender.send(Ok(TransportResponse::Message(message))));

                data_parsed
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
    waker: &ThreadWaker,
) {
    let mut buffer = Vec::with_capacity(INIT_BUFFER_CAPACITY);

    while let Ok(ref msg) = receiver.recv() {
        let mut cursor = Cursor::new(buffer);

        let res = MessageComponent::write(msg, &mut cursor);
        buffer = cursor.into_inner();

        match res {
            Ok(_) =>
                if let Err(error) = (&*stream).write_all(&buffer) {
                    let _ = sender.send(Err(TransportError::Fatal {
                        source: Source::WriteReliable,
                        error,
                    }));
                    return;
                },
            Err(error) => {
                let res = sender.send(Err(TransportError::Recoverable {
                    source: Source::WriteReliable,
                    error,
                }));
                return_if_err!(res);
                waker.wake();
            }
        }

        buffer.clear();
    }
}
