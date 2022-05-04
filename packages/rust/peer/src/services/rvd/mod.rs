mod client;
mod host;

pub use client::*;
pub use host::*;

use crate::services::InformEvent;
use common::messages::rvd::{ClipboardMeta, ClipboardNotification, ClipboardType, RvdMessage};

// most of RVD messages result in purely external changes. As such, RVD emits events for almost all messages. It is the job of the caller to respond to these events
pub enum RvdHandler {
    Host(RvdHostHandler),
    Client(RvdClientHandler),
}

impl RvdHandler {
    pub fn new_host() -> Self {
        Self::Host(RvdHostHandler::new())
    }

    pub fn new_client() -> Self {
        Self::Client(RvdClientHandler::new())
    }

    pub fn handle(
        &mut self,
        msg: RvdMessage,
        write: &mut Vec<RvdMessage>,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdError> {
        match self {
            Self::Host(handler) => handler.handle(msg, events)?,
            Self::Client(handler) => handler.handle(msg, write, events)?,
        }
        Ok(())
    }

    pub fn clipboard_data(
        data: Option<Vec<u8>>, // on input Option refers to whether the content exists
        is_content: bool,
        clipboard_type: ClipboardType,
    ) -> RvdMessage {
        RvdMessage::ClipboardNotification(ClipboardNotification {
            info: ClipboardMeta {
                clipboard_type,
                content_request: is_content,
            },
            type_exists: data.is_some(),
            // but here it it's only Some if (a) the type exists AND (b) it is a content request
            content: if is_content { data } else { None },
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RvdError {
    #[error("host error: {0}")]
    Host(#[from] RvdHostError),
    #[error("client error: {0}")]
    Client(#[from] RvdClientError),
}
