use crate::services::helpers::cipher_reliable_peer::{CipherError, CipherReliablePeer};
use crate::services::helpers::cipher_unreliable_peer::CipherUnreliablePeer;
use common::messages::wpskka::WpskkaMessage;

#[derive(Copy, Clone, Debug)]
enum State {
    Handshake,
    Data,
}

pub struct WpskkaHostHandler {
    state: State,
    reliable: Option<CipherReliablePeer>,
    unreliable: Option<CipherUnreliablePeer>,
}

impl WpskkaHostHandler {
    pub fn new() -> Self {
        Self {
            state: State::Handshake,
            reliable: None,
            unreliable: None,
        }
    }

    pub fn handle(&mut self, msg: WpskkaMessage) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match self.state {
            State::Handshake => match msg {
                WpskkaMessage::ClientHello(msg) => {
                    // TODO srp key derivation
                    self.state = State::Data;
                    Ok(None)
                }
                _ => Err(WpskkaHostError::WrongMessageForState(msg, self.state)),
            },
            State::Data => match msg {
                WpskkaMessage::TransportDataMessageReliable(msg) => {
                    let reliable = self.reliable.as_mut().unwrap();
                    Ok(Some(
                        reliable
                            .decrypt(msg.data)
                            .map_err(WpskkaHostError::CipherError)?,
                    ))
                }
                WpskkaMessage::TransportDataMessageUnreliable(msg) => {
                    let unreliable = self.unreliable.as_mut().unwrap();
                    Ok(Some(
                        unreliable
                            .decrypt(msg.data, msg.counter)
                            .map_err(WpskkaHostError::CipherError)?,
                    ))
                }
                _ => Err(WpskkaHostError::WrongMessageForState(msg, self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaHostError {
    #[error("{0}")]
    CipherError(CipherError),
    #[error("{0}")]
    SrpError(String), // TODO
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(WpskkaMessage, State),
}
