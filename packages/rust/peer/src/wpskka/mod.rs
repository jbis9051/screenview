pub mod auth;
mod client;
mod host;

pub use client::*;
pub use host::*;

use super::helpers::cipher_reliable_peer::CipherError;
use crate::{
    helpers::{
        cipher_reliable_peer::CipherReliablePeer,
        cipher_unreliable_peer::CipherUnreliablePeer,
    },
    InformEvent,
};
use common::messages::{
    wpskka::{TransportDataMessageReliable, TransportDataMessageUnreliable, WpskkaMessage},
    Data,
};
use std::{borrow::Cow, sync::Arc};


pub trait WpskkaHandlerTrait {
    fn handle(
        &mut self,
        msg: WpskkaMessage<'_>,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaError>;

    fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer>;

    fn wrap_unreliable(
        msg: Vec<u8>,
        cipher: &CipherUnreliablePeer,
    ) -> Result<TransportDataMessageUnreliable<'static>, CipherError> {
        let (data, counter) = cipher.encrypt(&msg)?;
        Ok(TransportDataMessageUnreliable {
            counter,
            data: Data(Cow::Owned(data)),
        })
    }

    fn reliable_cipher_mut(&mut self) -> &mut CipherReliablePeer;

    fn wrap_reliable(
        &mut self,
        msg: Vec<u8>,
    ) -> Result<TransportDataMessageReliable<'static>, CipherError> {
        let cipher = self.reliable_cipher_mut();

        Ok(TransportDataMessageReliable {
            data: Data(Cow::Owned(cipher.encrypt(&msg)?)),
        })
    }
}


#[derive(Debug, thiserror::Error)]
pub enum WpskkaError {
    #[error("host error: {0}")]
    Host(#[from] WpskkaHostError),
    #[error("client error: {0}")]
    Client(#[from] WpskkaClientError),
}


/*
impl WpskkaHandler {
    pub fn new_host() -> Self {
        Self::Host(WpskkaHostHandler::new())
    }

    pub fn new_client() -> Self {
        Self::Client(WpskkaClientHandler::new())
    }

    pub fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer> {
        match self {
            WpskkaHandler::Host(handler) => handler.unreliable_cipher(),
            WpskkaHandler::Client(handler) => handler.unreliable_cipher(),
        }
    }

    pub fn wrap_unreliable(
        msg: Vec<u8>,
        cipher: &CipherUnreliablePeer,
    ) -> Result<TransportDataMessageUnreliable<'static>, CipherError> {
        let (data, counter) = cipher.encrypt(&msg)?;
        Ok(TransportDataMessageUnreliable {
            counter,
            data: Data(Cow::Owned(data)),
        })
    }

    pub fn wrap_reliable(
        &mut self,
        msg: Vec<u8>,
    ) -> Result<TransportDataMessageReliable<'static>, CipherError> {
        let cipher = match self {
            WpskkaHandler::Host(handler) => handler.reliable_cipher_mut(),
            WpskkaHandler::Client(handler) => handler.reliable_cipher_mut(),
        };

        Ok(TransportDataMessageReliable {
            data: Data(Cow::Owned(cipher.encrypt(&msg)?)),
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
*/
