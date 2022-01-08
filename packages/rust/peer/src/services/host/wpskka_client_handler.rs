use crate::services::helpers::cipher_reliable_peer::{CipherError, CipherReliablePeer};
use crate::services::helpers::cipher_unreliable_peer::CipherUnreliablePeer;
use common::messages::wpskka::WpskkaMessage;

#[derive(Copy, Clone, Debug)]
pub enum State {
    PreHello,
    PreVerify,
    Data,
}

pub struct WpskkaClientHandler {
    state: State,
    reliable: Option<CipherReliablePeer>,
    unreliable: Option<CipherUnreliablePeer>,
}

impl WpskkaClientHandler {
    pub fn new() -> Self {
        Self {
            state: State::PreHello,
            reliable: None,
            unreliable: None,
        }
    }

    pub fn handle(&mut self, msg: WpskkaMessage) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        match self.state {
            State::PreHello => match msg {
                WpskkaMessage::HostHello(msg) => {
                    // TODO srp
                    self.state = State::PreVerify;
                    Ok(None)
                }
                _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
            },
            State::PreVerify => match msg {
                WpskkaMessage::HostVerify(msg) => {
                    // TODO srp
                    self.state = State::Data;
                    Ok(None)
                }
                _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
            },
            State::Data => match msg {
                WpskkaMessage::TransportDataMessageReliable(msg) => {
                    let reliable = self.reliable.as_mut().unwrap();
                    Ok(Some(
                        reliable
                            .decrypt(msg.data)
                            .map_err(WpskkaClientError::CipherError)?,
                    ))
                }
                WpskkaMessage::TransportDataMessageUnreliable(msg) => {
                    let unreliable = self.unreliable.as_mut().unwrap();
                    Ok(Some(
                        unreliable
                            .decrypt(msg.data, msg.counter)
                            .map_err(WpskkaClientError::CipherError)?,
                    ))
                }
                _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaClientError {
    #[error("{0}")]
    CipherError(CipherError),
    #[error("{0}")]
    SrpError(String), // TODO
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(WpskkaMessage, State),
}
