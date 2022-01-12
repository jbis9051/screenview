use crate::services::helpers::cipher_reliable_peer::{CipherError, CipherReliablePeer};
use crate::services::helpers::cipher_unreliable_peer::CipherUnreliablePeer;
use common::messages::sel::SelMessage;

#[derive(Copy, Clone, Debug)]
pub enum State {
    Handshake,
    Data,
}

pub struct SelHandler {
    state: State,
    reliable: Option<CipherReliablePeer>,
    unreliable: Option<CipherUnreliablePeer>,
}

impl SelHandler {
    pub fn new() -> Self {
        Self {
            state: State::Handshake,
            reliable: None,
            unreliable: None,
        }
    }

    pub fn handle(&mut self, msg: SelMessage) -> Result<Option<Vec<u8>>, Sel> {
        match self.state {
            State::Handshake => match msg {
                SelMessage::ServerHello(msg) => {
                    // TODO tls validation
                    self.state = State::Data;
                    Ok(None)
                }
                _ => Err(Sel::WrongMessageForState(msg, self.state)),
            },
            State::Data => match msg {
                SelMessage::TransportDataMessageReliable(msg) => {
                    let reliable = self.reliable.as_mut().unwrap();
                    Ok(Some(reliable.decrypt(msg.data).map_err(Sel::CipherError)?))
                }
                SelMessage::TransportDataPeerMessageUnreliable(msg) => {
                    let unreliable = self.unreliable.as_mut().unwrap();
                    Ok(Some(
                        unreliable
                            .decrypt(msg.data, msg.counter)
                            .map_err(Sel::CipherError)?,
                    ))
                }
                _ => Err(Sel::WrongMessageForState(msg, self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Sel {
    #[error("{0}")]
    CipherError(CipherError),
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(SelMessage, State),
}
