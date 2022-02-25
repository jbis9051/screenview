mod client;
mod host;

pub use client::*;
pub use host::*;

use crate::services::InformEvent;
use common::messages::rvd::RvdMessage;
use native::api::NativeApiTemplate;

pub enum RvdHandler<T: NativeApiTemplate> {
    Host(RvdHostHandler<T>),
    Client(RvdClientHandler<T>),
}

impl<T: NativeApiTemplate> RvdHandler<T> {
    pub fn handle(
        &mut self,
        msg: RvdMessage,
        write: &mut Vec<RvdMessage>,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdError<T>> {
        match self {
            Self::Host(handler) => handler.handle(msg, write, events)?,
            Self::Client(handler) => handler.handle(msg, write, events)?,
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RvdError<T: NativeApiTemplate> {
    #[error("host error: {0}")]
    Host(#[from] RvdHostError<T>),
    #[error("client error: {0}")]
    Client(#[from] RvdClientError<T>),
}
