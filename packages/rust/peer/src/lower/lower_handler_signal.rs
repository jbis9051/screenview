use crate::{
    io::LENGTH_FIELD_WIDTH,
    lower::{LowerError, LowerHandlerTrait, LowerSendError},
    sel_handler::SelHandler,
    svsc_handler::SvscHandler,
    ChanneledMessage,
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
        message: ChanneledMessage<SvscMessage<'_>>,
    ) -> Result<ChanneledMessage<Vec<u8>>, LowerSendError> {
        let sel = match message.map(|msg| msg.to_bytes(None)) {
            ChanneledMessage::Reliable(message) => SelHandler::wrap_reliable(message?),
            ChanneledMessage::Unreliable(message) => {
                let cipher = self.sel.unreliable_cipher();
                SelHandler::wrap_unreliable(message?, *self.svsc.peer_id().unwrap(), cipher)?
            }
        };

        match &sel {
            SelMessage::TransportDataMessageReliable(..) =>
                self.send_sel(&sel).map(ChanneledMessage::Reliable),
            SelMessage::TransportDataPeerMessageUnreliable(..)
            | SelMessage::TransportDataServerMessageUnreliable(..) =>
                self.send_sel(&sel).map(ChanneledMessage::Unreliable),
        }
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
            // TODO check if reliable
            match self.send_svsc(ChanneledMessage::Reliable(message))? {
                ChanneledMessage::Reliable(data) => send_reliable.push(data),
                ChanneledMessage::Unreliable(data) => send_unreliable.push(data),
            }
        }

        Ok(events)
    }

    fn send(
        &mut self,
        wpskka_bytes: ChanneledMessage<Vec<u8>>,
    ) -> Result<ChanneledMessage<Vec<u8>>, LowerError> {
        self.send_svsc(
            wpskka_bytes.map(|data| SvscMessage::SessionDataSend(SvscHandler::wrap(data))),
        )
        .map_err(Into::into)
    }
}
