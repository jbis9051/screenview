use crate::{
    debug,
    hash,
    helpers::{
        cipher_reliable_peer::CipherError,
        cipher_unreliable_peer::CipherUnreliablePeer,
        crypto::kdf2,
    },
};
use common::{
    constants::{SEL_AEAD_CONTEXT, SEL_KDF_CONTEXT},
    messages::{
        sel::{SelMessage, TransportDataMessageReliable, TransportDataPeerMessageUnreliable},
        svsc::PeerId,
        Data,
    },
};
use std::borrow::Cow;

pub struct SelHandler {
    unreliable: Option<CipherUnreliablePeer>,
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

    pub fn unreliable_cipher(&mut self) -> &mut CipherUnreliablePeer {
        self.unreliable.as_mut().unwrap()
    }

    pub fn wrap_reliable(msg: Vec<u8>) -> SelMessage<'static> {
        SelMessage::TransportDataMessageReliable(TransportDataMessageReliable {
            data: Data(Cow::Owned(msg)),
        })
    }

    // this is static because we need to access it in multiple threads and we don't want to Arc sel_handler, instead we just accept cipher which is Arced in self.unreliable.unwrap()
    pub fn wrap_unreliable(
        msg: Vec<u8>,
        peer_id: PeerId,
        cipher: &mut CipherUnreliablePeer,
    ) -> Result<SelMessage<'static>, CipherError> {
        let (data, counter) = cipher.encrypt(&msg)?;
        Ok(SelMessage::TransportDataPeerMessageUnreliable(
            TransportDataPeerMessageUnreliable {
                peer_id,
                counter,
                data: Data(Cow::Owned(data)),
            },
        ))
    }

    /// Warning: This resets unreliable
    pub fn derive_unreliable(&mut self, session_id: &[u8], peer_id: &[u8], peer_key: &[u8]) {
        let (send_key, receive_key) =
            kdf2(&[session_id, peer_id, peer_key].concat(), SEL_KDF_CONTEXT);
        // TODO Security: Look into zeroing out the data here
        self.unreliable = Some(CipherUnreliablePeer::new(
            send_key.to_vec(),
            receive_key.to_vec(),
            SEL_AEAD_CONTEXT.to_vec(),
        ));
    }

    pub fn handle<'a>(&mut self, msg: &'a SelMessage<'_>) -> Result<Cow<'a, [u8]>, SelError> {
        match msg {
            SelMessage::TransportDataMessageReliable(msg) => Ok(Cow::Borrowed(&*msg.data.0)),
            SelMessage::TransportDataServerMessageUnreliable(msg) => {
                let unreliable = self.unreliable.as_mut().unwrap(); // TODO do we want to panic here?
                Ok(unreliable
                    .decrypt(&*msg.data.0, msg.counter)
                    .map(Cow::Owned)
                    .map_err(SelError::CipherError)?)
            }
            _ => Err(SelError::WrongMessage(debug(msg))),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SelError {
    #[error("{0}")]
    CipherError(#[from] CipherError),
    #[error("invalid message {0}")]
    WrongMessage(String),
}
