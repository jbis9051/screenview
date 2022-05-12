use crate::{
    helpers::cipher_reliable_peer::CipherError,
    higher_handler::HigherOutput,
    sel_handler::SelError,
    svsc_handler::SvscError,
    ChanneledMessage,
    InformEvent,
};
use common::messages::{
    sel::SelMessage,
    svsc::{LeaseId, SvscMessage},
    Error as MessageComponentError,
};

mod lower_handler_direct;
mod lower_handler_signal;

pub use lower_handler_direct::*;
pub use lower_handler_signal::*;

pub trait LowerHandlerTrait {
    // takes messages from the wire and outputs wpskka bytes, send is bytes to put back on the wire
    fn handle(
        &mut self,
        wire: &[u8],
        output: &mut Vec<u8>,
        send: &mut Vec<ChanneledMessage<Vec<u8>>>,
    ) -> Result<Vec<InformEvent>, LowerError>;

    // takes wpskka bytes and outputs messages to the wire
    fn send<'a, M: Into<sealed::LowerMessage<'a>>>(
        &mut self,
        message: M,
    ) -> Result<ChanneledMessage<Vec<u8>>, LowerError>;

    fn establish_session_request(
        &mut self,
        lease_id: LeaseId,
    ) -> ChanneledMessage<SvscMessage<'static>>;
}

pub(crate) mod sealed {
    use super::*;

    pub enum LowerMessage<'a> {
        HigherOutput(ChanneledMessage<HigherOutput>),
        Svsc(ChanneledMessage<SvscMessage<'a>>),
        Sel(SelMessage<'a>),
    }
}

impl From<ChanneledMessage<HigherOutput>> for sealed::LowerMessage<'_> {
    fn from(output: ChanneledMessage<HigherOutput>) -> Self {
        Self::HigherOutput(output)
    }
}

impl<'a> From<ChanneledMessage<SvscMessage<'a>>> for sealed::LowerMessage<'a> {
    fn from(msg: ChanneledMessage<SvscMessage<'a>>) -> Self {
        Self::Svsc(msg)
    }
}

impl<'a> From<SelMessage<'a>> for sealed::LowerMessage<'a> {
    fn from(msg: SelMessage<'a>) -> Self {
        Self::Sel(msg)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LowerSendError {
    #[error("failed to encode message: {0}")]
    Encode(#[from] MessageComponentError),
    #[error("failed to encrypt message: {0}")]
    Cipher(#[from] CipherError),
}

#[derive(Debug, thiserror::Error)]
pub enum LowerError {
    #[error("failed to decode message: {0}")]
    Decode(#[from] MessageComponentError),
    #[error("SEL handler error: {0}")]
    Sel(#[from] SelError),
    #[error("SVSC handler error: {0}")]
    Svsc(#[from] SvscError),
    #[error("send error: {0}")]
    SendError(#[from] LowerSendError),
}
