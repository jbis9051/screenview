mod auth;
mod client;
mod host;

pub use client::*;
pub use host::*;

use super::helpers::cipher_reliable_peer::CipherError;
use crate::services::{
    helpers::cipher_unreliable_peer::CipherUnreliablePeer,
    InformEvent,
    SendError,
};
use common::messages::wpskka::{
    TransportDataMessageReliable,
    TransportDataMessageUnreliable,
    WpskkaMessage,
};
use std::sync::Arc;

pub enum WpskkaHandler {
    Host(WpskkaHostHandler),
    Client(WpskkaClientHandler),
}

impl WpskkaHandler {
    pub fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer> {
        match self {
            WpskkaHandler::Host(handler) => handler.unreliable_cipher(),
            WpskkaHandler::Client(handler) => handler.unreliable_cipher(),
        }
    }

    pub fn wrap_unreliable(
        msg: Vec<u8>,
        cipher: &CipherUnreliablePeer,
    ) -> Result<TransportDataMessageUnreliable, CipherError> {
        let (data, counter) = cipher.encrypt(msg)?;
        Ok(TransportDataMessageUnreliable { counter, data })
    }

    pub fn wrap_reliable(
        &mut self,
        msg: Vec<u8>,
    ) -> Result<TransportDataMessageReliable, CipherError> {
        let cipher = match self {
            WpskkaHandler::Host(handler) => handler.reliable_cipher_mut(),
            WpskkaHandler::Client(handler) => handler.reliable_cipher_mut(),
        };

        Ok(TransportDataMessageReliable {
            data: cipher.encrypt(msg)?,
        })
    }

    pub fn handle(
        &mut self,
        msg: WpskkaMessage,
        write: &mut Vec<WpskkaMessage>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaError> {
        let data = match self {
            Self::Host(handler) => handler.handle(msg, write, events)?,
            Self::Client(handler) => handler.handle(msg, write, events)?,
        };
        Ok(data)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaError {
    #[error("host error: {0}")]
    Host(#[from] WpskkaHostError),
    #[error("client error: {0}")]
    Client(#[from] WpskkaClientError),
}
