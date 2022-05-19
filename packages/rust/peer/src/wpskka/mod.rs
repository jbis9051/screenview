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
