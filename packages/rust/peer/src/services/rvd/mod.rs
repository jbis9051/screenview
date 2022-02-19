mod client;
mod host;

pub use client::*;
pub use host::*;

use crate::services::{InformEvent, SendError};
use common::messages::rvd::RvdMessage;

pub enum RvdHandler {
    Host(RvdHostHandler),
    Client(RvdClientHandler),
}

impl RvdHandler {
    pub fn handle<F>(
        &mut self,
        msg: RvdMessage,
        write: F,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdError>
    where
        F: FnMut(RvdMessage) -> Result<(), SendError>,
    {
        match self {
            Self::Host(handler) => handler.handle(msg, write)?,
            Self::Client(handler) => handler.handle(msg, write, events)?,
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RvdError {
    #[error("host error: {0}")]
    Host(#[from] RvdHostError),
    #[error("client error: {0}")]
    Client(#[from] RvdClientError),
}
