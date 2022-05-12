use super::sealed::LowerMessage;
use crate::{
    lower::{LowerError, LowerHandlerTrait, LowerSendError},
    sel_handler::SelHandler,
    svsc_handler::SvscHandler,
    ChanneledMessage,
    InformEvent,
};
use common::messages::{
    sel::SelMessage,
    svsc::{LeaseId, SvscMessage},
    Message,
    MessageComponent,
};
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
        let sel = match message.map(|msg| msg.to_bytes()) {
            ChanneledMessage::Reliable(message) => SelHandler::wrap_reliable(message?),
            ChanneledMessage::Unreliable(message) => {
                let cipher = self.sel.unreliable_cipher();
                SelHandler::wrap_unreliable(message?, *self.svsc.peer_id().unwrap(), cipher)?
            }
        };

        self.send_sel(&sel)
    }

    fn send_sel(
        &mut self,
        sel: &SelMessage<'_>,
    ) -> Result<ChanneledMessage<Vec<u8>>, LowerSendError> {
        let bytes = sel.to_bytes()?;

        match &sel {
            SelMessage::TransportDataMessageReliable(..) => Ok(ChanneledMessage::Reliable(bytes)),
            SelMessage::TransportDataPeerMessageUnreliable(..)
            | SelMessage::TransportDataServerMessageUnreliable(..) =>
                Ok(ChanneledMessage::Unreliable(bytes)),
        }
    }
}

impl LowerHandlerTrait for LowerHandlerSignal {
    fn handle(
        &mut self,
        wire: &[u8],
        output: &mut Vec<u8>,
        send: &mut Vec<ChanneledMessage<Vec<u8>>>,
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
            send.push(self.send_svsc(ChanneledMessage::Reliable(message))?);
        }

        Ok(events)
    }

    fn send<'a, M: Into<LowerMessage<'a>>>(
        &mut self,
        message: M,
    ) -> Result<ChanneledMessage<Vec<u8>>, LowerError> {
        let result = match message.into() {
            LowerMessage::HigherOutput(output) => self.send_svsc(
                output.map(|data| SvscMessage::SessionDataSend(SvscHandler::wrap(data.0))),
            ),
            LowerMessage::Svsc(svsc) => self.send_svsc(svsc),
            LowerMessage::Sel(ref sel) => self.send_sel(sel),
        };

        result.map_err(Into::into)
    }

    fn establish_session_request(
        &mut self,
        lease_id: LeaseId,
    ) -> ChanneledMessage<SvscMessage<'static>> {
        ChanneledMessage::Reliable(self.svsc.establish_session_request(lease_id))
    }
}
