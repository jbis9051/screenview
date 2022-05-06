use crate::{
    higher_handler::{HigherError, HigherHandler, HigherHandlerTrait},
    io::{IoHandle, Reliable, TransportError, TransportResponse, Unreliable},
    lower::{LowerError, LowerHandlerTrait},
    InformEvent,
};

pub struct ScreenViewHandler<H, L, R, U> {
    higher: H,
    lower: L,
    io_handle: IoHandle<R, U>,
}

impl<H: HigherHandlerTrait, L: LowerHandlerTrait, R: Reliable, U: Unreliable>
    ScreenViewHandler<H, L, R, U>
{
    pub fn new(higher: H, lower: L, io_handle: IoHandle<R, U>) -> Self {
        Self {
            higher,
            lower,
            io_handle,
        }
    }

    pub fn io_handle(&mut self) -> &mut IoHandle<R, U> {
        &mut self.io_handle
    }

    pub fn handle_next_message(&mut self) -> Option<Result<Vec<InformEvent>, HandlerError>> {
        let result = self.io_handle.recv()?;
        Some(match result {
            Ok(TransportResponse::Message(message)) => self.handle(&message),
            Ok(TransportResponse::Shutdown(source)) =>
                Ok(vec![InformEvent::TransportShutdown(source)]),
            Err(error) => Err(error.into()),
        })
    }

    pub fn handle(&mut self, wire: &[u8]) -> Result<Vec<InformEvent>, HandlerError> {
        // in lower: wire(wpskaa | sel) --> wpskka
        // in higher: wpskka --> wpskaa
        // out lower: wpskak --> wire(wpskaa | sel)

        // if we are direct, wire is wpskaa data
        // if we are signal, wire data is sel

        let mut send_reliable = Vec::new();
        let mut send_unreliable = Vec::new();
        let mut inform_events = Vec::new();


        let mut data_for_higher = Vec::new();
        // if direct, lower just passes us back
        // if signal it decrypts it and passes is through svsc
        // regardless we will get wpskka data out of ths
        inform_events.append(&mut self.lower.handle(
            wire,
            &mut data_for_higher,
            &mut send_reliable,
            &mut send_unreliable,
        )?);

        for msg in send_reliable {
            self.io_handle
                .send_reliable(msg)
                .map_err(|_| HandlerError::SendReliable)?;
        }

        for msg in send_unreliable {
            self.io_handle
                .send_unreliable(msg)
                .map_err(|_| HandlerError::SendUnreliable)?;
        }

        // we get wpskka data from higher
        let mut wpskka_outputs = Vec::new();
        inform_events.append(&mut self.higher.handle(&data_for_higher, &mut wpskka_outputs)?);


        let mut to_wires = Vec::new();

        for (wpskka_output, reliable) in wpskka_outputs {
            to_wires.push(self.lower.send(wpskka_output, reliable)?);
        }

        for (wire_msg, reliable) in to_wires {
            if reliable {
                self.io_handle
                    .send_reliable(wire_msg)
                    .map_err(|_| HandlerError::SendReliable)?;
            } else {
                self.io_handle
                    .send_unreliable(wire_msg)
                    .map_err(|_| HandlerError::SendUnreliable)?;
            }
        }

        Ok(inform_events)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("transport-layer error: {0}")]
    Transport(#[from] TransportError),
    #[error("lower-layer error: {0}")]
    Lower(#[from] LowerError),
    #[error("higher-layer error: {0}")]
    Higher(#[from] HigherError),
    #[error("send-reliable error")]
    SendReliable,
    #[error("send-unreliable error")]
    SendUnreliable,
}
