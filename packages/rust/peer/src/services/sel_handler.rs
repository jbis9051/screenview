use crate::services::helpers::cipher_reliable_peer::{CipherError, CipherReliablePeer};
use crate::services::helpers::cipher_unreliable_peer::CipherUnreliablePeer;
use common::messages::sel::SelMessage;

#[derive(Copy, Clone, Debug)]
pub enum State {
    Data,
}

pub struct SelHandler {
    state: State,
    unreliable: Option<CipherUnreliablePeer>,
    // reliable is TLS and is handled elsewhere
}

impl SelHandler {
    pub fn new() -> Self {
        Self {
            state: State::Data,
            unreliable: None,
        }
    }

    pub fn handle(&mut self, msg: SelMessage) -> Result<Option<Vec<u8>>, Sel> {
        match self.state {
            State::Data => match msg {
                SelMessage::TransportDataMessageReliable(msg) => Ok(Some(msg.data)),
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
