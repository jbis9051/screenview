use crate::{
    io::LENGTH_FIELD_WIDTH,
    lower::{LowerError, LowerHandlerOutput, LowerHandlerTrait, LowerSendError},
    sel_handler::SelHandler,
    svsc_handler::SvscHandler,
    InformEvent,
};
use common::messages::{sel::SelMessage, svsc::SvscMessage, MessageComponent};
use std::io::Cursor;

// we handle the case when we are using a signal server

pub struct LowerHandlerSignal {
    sel: SelHandler,
    svsc: SvscHandler,
}

impl LowerHandlerSignal {
    pub fn new() -> Self {
        Self {
            sel: SelHandler::new(),
            svsc: SvscHandler::new(),
        }
    }

    fn send_svsc(
        &mut self,
        message: SvscMessage<'_>,
        reliable: bool,
    ) -> Result<LowerHandlerOutput, LowerSendError> {
        let data = message.to_bytes(None)?;

        let sel = if reliable {
            SelHandler::wrap_reliable(data)
        } else {
            let cipher = self.sel.unreliable_cipher();
            SelHandler::wrap_unreliable(data, *self.svsc.peer_id().unwrap(), cipher)?
        };

        Ok((
            self.send_sel(&sel)?,
            !matches!(sel, SelMessage::TransportDataPeerMessageUnreliable(_)),
        ))
    }

    fn send_sel(&mut self, sel: &SelMessage<'_>) -> Result<Vec<u8>, LowerSendError> {
        Ok(sel.to_bytes(Some(LENGTH_FIELD_WIDTH))?)
    }
}

impl LowerHandlerTrait for LowerHandlerSignal {
    fn handle(
        &mut self,
        wire: &[u8],
        output: &mut Vec<u8>,
        send_reliable: &mut Vec<Vec<u8>>,
        send_unreliable: &mut Vec<Vec<u8>>,
    ) -> Result<Vec<InformEvent>, LowerError> {
        let message = SelMessage::read(&mut Cursor::new(wire))?;

        let mut events = Vec::new();
        let svsc_data = self.sel.handle(&message)?;
        let svsc_message = SvscMessage::read(&mut Cursor::new(&svsc_data))?;
        let mut send_svsc = Vec::new();

        match self
            .svsc
            .handle(svsc_message, &mut send_svsc, &mut events)?
        {
            Some(data) => {
                output.extend_from_slice(&data);
            }
            None => {}
        };

        for message in send_svsc {
            let (data, reliable) = self.send_svsc(message, true)?; // TODO check if reliable
            if reliable {
                send_reliable.push(data);
            } else {
                send_unreliable.push(data);
            }
        }

        Ok(events)
    }

    fn send(
        &mut self,
        wpskka_bytes: Vec<u8>,
        reliable: bool,
    ) -> Result<LowerHandlerOutput, LowerError> {
        let svsc = SvscHandler::wrap(wpskka_bytes);
        Ok(self.send_svsc(SvscMessage::SessionDataSend(svsc), reliable)?)
    }
}
