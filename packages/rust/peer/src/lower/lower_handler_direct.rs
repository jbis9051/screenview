use crate::{
    lower::{LowerError, LowerHandlerTrait},
    InformEvent,
};
use common::messages::ChanneledMessage;

use super::sealed::LowerMessage;

// we handle the case when we are going directly to the peer, we just pass things through

pub struct LowerHandlerDirect;

impl LowerHandlerDirect {
    pub fn new() -> Self {
        Self
    }
}

impl LowerHandlerTrait for LowerHandlerDirect {
    fn handle(
        &mut self,
        wire: &[u8],
        output: &mut Vec<u8>,
        _send: &mut Vec<ChanneledMessage<Vec<u8>>>,
    ) -> Result<Vec<InformEvent>, LowerError> {
        output.extend_from_slice(wire);
        Ok(Vec::new())
    }

    fn send<'a, M: Into<LowerMessage<'a>>>(
        &mut self,
        message: M,
    ) -> Result<ChanneledMessage<Vec<u8>>, LowerError> {
        match message.into() {
            LowerMessage::HigherOutput(output) => Ok(output.map(|wrapper| wrapper.0)),
            LowerMessage::Svsc(..) => panic!("Attempted to send SVSC message to direct handler"),
            LowerMessage::Sel(..) => panic!("Attempted to send SEL message to direct handler"),
        }
    }
}
