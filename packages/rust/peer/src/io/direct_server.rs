use common::event_loop::{JoinOnDrop, ThreadWaker};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    io::{self, ErrorKind},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub struct DirectServer {
    running: Arc<AtomicBool>,
    incoming: Receiver<Result<TcpStream, io::Error>>,
    _handle: JoinOnDrop<()>,
}

impl DirectServer {
    pub fn new<A: ToSocketAddrs>(addr: A, waker: ThreadWaker) -> Result<Self, io::Error> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = unbounded();
        let handle = thread::spawn({
            let running = Arc::clone(&running);
            move || listen(listener, tx, running, waker)
        });

        Ok(Self {
            running,
            incoming: rx,
            _handle: JoinOnDrop::new(handle),
        })
    }

    pub fn recv(&self) -> Option<Result<TcpStream, io::Error>> {
        self.incoming.try_recv().ok()
    }
}

impl Drop for DirectServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn listen(
    listener: TcpListener,
    sender: Sender<Result<TcpStream, io::Error>>,
    running: Arc<AtomicBool>,
    waker: ThreadWaker,
) {
    for stream in listener.incoming() {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        match stream {
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }

            _ => {
                let _ = sender.send(stream);
            }
        }
    }

    waker.wake();
}
