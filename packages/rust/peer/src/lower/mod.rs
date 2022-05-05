use crate::{
    helpers::cipher_reliable_peer::CipherError,
    sel_handler::SelError,
    svsc_handler::SvscError,
    InformEvent,
};
use common::messages::Error as MessageComponentError;

pub mod lower_handler_direct;
pub mod lower_handler_signal;

type LowerHandlerOutput = (Vec<u8>, bool);

trait LowerHandlerTrait {
    // takes messages from the wire and outputs wpskka bytes, send is bytes to put back on the wire
    fn handle(
        &mut self,
        wire: &[u8],
        output: &mut Vec<u8>,
        send_reliable: &mut Vec<u8>,
        send_unreliable: &mut Vec<Vec<u8>>,
    ) -> Result<Vec<InformEvent>, LowerError>;

    // takes wpskka bytes and outputs messages to the wire
    fn send(
        &mut self,
        wpskka_bytes: Vec<u8>,
        reliable: bool,
    ) -> Result<LowerHandlerOutput, LowerError>;
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
