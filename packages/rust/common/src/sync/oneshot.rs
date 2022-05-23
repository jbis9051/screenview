use crossbeam_channel::{self, bounded, RecvError, SendError};

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = bounded(1);
    (Sender { sender }, Receiver { receiver })
}

pub struct Sender<T> {
    sender: crossbeam_channel::Sender<T>,
}

impl<T> Sender<T> {
    pub fn send(self, msg: T) -> Result<(), SendError<T>> {
        self.sender.send(msg)
    }
}

pub struct Receiver<T> {
    receiver: crossbeam_channel::Receiver<T>,
}

impl<T> Receiver<T> {
    pub fn recv(self) -> Result<T, RecvError> {
        self.receiver.recv()
    }
}
