use crate::{
    helpers::cipher_reliable_peer::CipherError,
    sel_handler::SelError,
    svsc_handler::SvscError,
    ChanneledMessage,
    InformEvent,
};
use common::messages::Error as MessageComponentError;

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
        send_reliable: &mut Vec<Vec<u8>>, // this must be a double vec because we don't prepend length on to_bytes calls so we need a way to indicates the seperation of messages
        send_unreliable: &mut Vec<Vec<u8>>, // this must be a double vec because udp is not a stream (obviously)
    ) -> Result<Vec<InformEvent>, LowerError>;

    // takes wpskka bytes and outputs messages to the wire
    fn send(
        &mut self,
        wpskka_bytes: ChanneledMessage<Vec<u8>>,
    ) -> Result<ChanneledMessage<Vec<u8>>, LowerError>;
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
