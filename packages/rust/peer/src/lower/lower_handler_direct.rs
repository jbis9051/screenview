use crate::{
    lower::{LowerError, LowerHandlerOutput, LowerHandlerTrait},
    InformEvent,
};

// we handle the case when we are going directly to the peer, we just pass things through

struct LowerHandlerDirect;

impl LowerHandlerTrait for LowerHandlerDirect {
    fn handle(
        &mut self,
        wire: &[u8],
        output: &mut Vec<u8>,
        _send_reliable: &mut Vec<Vec<u8>>,
        _send_unreliable: &mut Vec<Vec<u8>>,
    ) -> Result<Vec<InformEvent>, LowerError> {
        output.extend_from_slice(wire);
        Ok(Vec::new())
    }

    fn send(
        &mut self,
        wpskka_bytes: Vec<u8>,
        reliable: bool,
    ) -> Result<LowerHandlerOutput, LowerError> {
        Ok((wpskka_bytes, reliable))
    }
}
