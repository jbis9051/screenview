use crate::{
    hash,
    helpers::{
        cipher_reliable_peer::CipherError,
        cipher_unreliable_peer::CipherUnreliablePeer,
        crypto::kdf2,
    },
};
use common::messages::{
    sel::{SelMessage, TransportDataMessageReliable, TransportDataPeerMessageUnreliable},
    svsc::PeerId,
};
use std::sync::Arc;

pub struct SelHandler {
    unreliable: Option<Arc<CipherUnreliablePeer>>,
    // reliable is TLS and is handled elsewhere
}

impl Default for SelHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SelHandler {
    pub fn new() -> Self {
        Self { unreliable: None }
    }

    pub fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer> {
        self.unreliable.as_ref().unwrap()
    }

    pub fn wrap_reliable(msg: Vec<u8>) -> SelMessage {
        SelMessage::TransportDataMessageReliable(TransportDataMessageReliable { data: msg })
    }

    // this is static because we need to access it in multiple threads and we don't want to Arc sel_handler, instead we just accept cipher which is Arced in self.unreliable.unwrap()
    pub fn wrap_unreliable(
        msg: Vec<u8>,
        peer_id: PeerId,
        cipher: &CipherUnreliablePeer,
    ) -> Result<SelMessage, CipherError> {
        let (data, counter) = cipher.encrypt(&msg)?;
        Ok(SelMessage::TransportDataPeerMessageUnreliable(
            TransportDataPeerMessageUnreliable {
                peer_id,
                counter,
                data,
            },
        ))
    }

    /// Warning: This resets unreliable
    pub fn derive_unreliable(&mut self, session_id: &[u8], peer_id: &[u8], peer_key: &[u8]) {
        let (send_key, receive_key) = kdf2(hash!(session_id, peer_id, peer_key));
        // TODO Security: Look into zeroing out the data here
        self.unreliable = Some(Arc::new(CipherUnreliablePeer::new(
            send_key.to_vec(),
            receive_key.to_vec(),
        )));
    }

    pub fn handle(&mut self, msg: SelMessage) -> Result<Vec<u8>, SelError> {
        match msg {
            SelMessage::TransportDataMessageReliable(msg) => Ok(msg.data),
            SelMessage::TransportDataServerMessageUnreliable(msg) => {
                let unreliable = self.unreliable.as_mut().unwrap(); // TODO do we want to panic here?
                Ok(unreliable
                    .decrypt(&msg.data, msg.counter)
                    .map_err(SelError::CipherError)?)
            }
            _ => Err(SelError::WrongMessage(Box::new(msg))),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SelError {
    #[error("{0}")]
    CipherError(#[from] CipherError),
    #[error("invalid message {0:?}")]
    WrongMessage(Box<SelMessage>),
}
