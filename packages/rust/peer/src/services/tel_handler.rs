use crate::services::helpers::tel_cipher_reliable_peer::{TelCipherError, TelCipherReliablePeer};
use crate::services::helpers::tel_cipher_unreliable_peer::TelCipherUnreliablePeer;
use common::messages::tel::TelMessage;

#[derive(Copy, Clone, Debug)]
enum State {
    Handshake,
    Data,
}

pub struct TelHandler {
    state: State,
    reliable: Option<TelCipherReliablePeer>,
    unreliable: Option<TelCipherUnreliablePeer>,
}

impl TelHandler {
    pub fn new() -> Self {
        Self {
            state: State::Handshake,
            reliable: None,
            unreliable: None,
        }
    }

    pub fn handle(&mut self, msg: TelMessage) -> Result<Option<Vec<u8>>, TelError> {
        match self.state {
            State::Handshake => match msg {
                TelMessage::ServerHello(msg) => {
                    // TODO tls validation
                    self.state = State::Data;
                    Ok(None)
                }
                _ => Err(TelError::WrongMessageForState(msg, self.state)),
            },
            State::Data => match msg {
                TelMessage::TransportDataMessageReliable(msg) => {
                    let reliable = self.reliable.as_mut().unwrap();
                    Ok(Some(
                        reliable.decrypt(msg.data).map_err(TelError::CipherError)?,
                    ))
                }
                TelMessage::TransportDataPeerMessageUnreliable(msg) => {
                    let unreliable = self.unreliable.as_mut().unwrap();
                    Ok(Some(
                        unreliable
                            .decrypt(msg.data, msg.counter)
                            .map_err(TelError::CipherError)?,
                    ))
                }
                _ => Err(TelError::WrongMessageForState(msg, self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TelError {
    #[error("{0}")]
    CipherError(TelCipherError),
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(TelMessage, State),
}
