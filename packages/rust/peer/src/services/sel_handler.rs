use crate::services::helpers::{
    cipher_reliable_peer::CipherError,
    cipher_unreliable_peer::CipherUnreliablePeer,
};
use common::messages::{
    sel::{SelMessage, TransportDataMessageReliable, TransportDataPeerMessageUnreliable},
    svsc::PeerId,
};
use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
pub enum State {
    Data,
}

pub struct SelHandler {
    state: State,
    unreliable: Option<Arc<CipherUnreliablePeer>>,
    // reliable is TLS and is handled elsewhere
}

impl SelHandler {
    pub fn new() -> Self {
        Self {
            state: State::Data,
            unreliable: None,
        }
    }

    pub fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer> {
        self.unreliable.as_ref().unwrap()
    }

    pub fn wrap_reliable(msg: Vec<u8>) -> TransportDataMessageReliable {
        TransportDataMessageReliable { data: msg }
    }

    pub fn wrap_unreliable(
        msg: Vec<u8>,
        peer_id: PeerId,
        cipher: &CipherUnreliablePeer,
    ) -> Result<TransportDataPeerMessageUnreliable, CipherError> {
        let (data, counter) = cipher.encrypt(msg)?;
        Ok(TransportDataPeerMessageUnreliable {
            peer_id,
            counter,
            data,
        })
    }

    pub fn handle(&mut self, msg: SelMessage) -> Result<Vec<u8>, SelError> {
        match self.state {
            State::Data => match msg {
                SelMessage::TransportDataMessageReliable(msg) => Ok(msg.data),
                SelMessage::TransportDataPeerMessageUnreliable(msg) => {
                    let unreliable = self.unreliable.as_mut().unwrap();
                    Ok(unreliable
                        .decrypt(msg.data, msg.counter)
                        .map_err(SelError::CipherError)?)
                }
                _ => Err(SelError::WrongMessageForState(Box::new(msg), self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SelError {
    #[error("{0}")]
    CipherError(#[from] CipherError),
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(Box<SelMessage>, State),
}
