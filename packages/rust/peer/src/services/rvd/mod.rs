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

    /// Option < // is it a content request
    ///   Option< // is there content
    ///       Vec<u8> // the content
    ///   >
    /// >
    pub fn clipboard_data(
        data: Option<Vec<u8>>,
        is_content: bool,
        clipboard_type: ClipboardType,
    ) -> RvdMessage {
        RvdMessage::ClipboardNotification(ClipboardNotification {
            info: ClipboardMeta {
                clipboard_type,
                content_request: is_content,
            },
            content: data,
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
