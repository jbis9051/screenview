use crate::{
    helpers::cipher_reliable_peer::CipherError,
    rvd::{
        RvdClientHandler,
        RvdError,
        RvdHandlerTrait,
        RvdHostError,
        RvdHostHandler,
        ShareDisplayResult,
    },
    wpskka::{WpskkaClientHandler, WpskkaError, WpskkaHandlerTrait, WpskkaHostHandler},
    InformEvent,
};
use common::messages::{
    rvd::{AccessMask, DisplayId, RvdMessage},
    wpskka::{TransportDataMessageUnreliable, WpskkaMessage},
    ChanneledMessage,
    Error as MessageComponentError,
    Message,
    MessageComponent,
};
use std::io::Cursor;

pub trait HigherHandlerTrait {
    // takes in wpskka data and ouputs a vector of wpskka data and reliable bool
    fn handle(
        &mut self,
        wpskka_data: &[u8],
        output: &mut Vec<ChanneledMessage<HigherOutput>>,
    ) -> Result<Vec<InformEvent>, HigherError>;

    // takes in
    fn send<'a, M: Into<sealed::HigherMessage<'a>>>(
        &mut self,
        message: M,
    ) -> Result<ChanneledMessage<HigherOutput>, HigherError>;
}

pub(crate) mod sealed {
    use super::*;

    pub enum HigherMessage<'a> {
        Rvd(RvdMessage),
        Wpskka(WpskkaMessage<'a>),
    }
}

impl From<RvdMessage> for sealed::HigherMessage<'_> {
    fn from(msg: RvdMessage) -> Self {
        Self::Rvd(msg)
    }
}

impl<'a> From<WpskkaMessage<'a>> for sealed::HigherMessage<'a> {
    fn from(msg: WpskkaMessage<'a>) -> Self {
        Self::Wpskka(msg)
    }
}

pub struct HigherOutput(pub(crate) Vec<u8>);

pub type HigherHandlerHost = HigherHandler<WpskkaHostHandler, RvdHostHandler>;
pub type HigherHandlerClient = HigherHandler<WpskkaClientHandler, RvdClientHandler>;

pub struct HigherHandler<Wpskka, Rvd> {
    wpskka: Wpskka,
    rvd: Rvd,
}

impl HigherHandlerHost {
    pub fn new() -> Self {
        HigherHandler {
            wpskka: WpskkaHostHandler::new(),
            rvd: RvdHostHandler::new(),
        }
    }

    pub fn set_static_password(&mut self, static_password: Option<Vec<u8>>) {
        self.wpskka.set_static_password(static_password)
    }

    pub fn share_display(
        &mut self,
        name: String,
        access: AccessMask,
    ) -> Result<(DisplayId, RvdMessage), RvdHostError> {
        self.rvd.share_display(name, access)
    }

    pub fn frame_update<'a>(&mut self, pkt: &[u8]) -> impl Iterator<Item = RvdMessage> + 'a {
        self.rvd.frame_update(pkt)
    }
}

impl HigherHandlerClient {
    pub fn new() -> Self {
        HigherHandler {
            wpskka: WpskkaClientHandler::new(),
            rvd: RvdClientHandler::new(),
        }
    }

    pub fn process_password(
        &mut self,
        password: &[u8],
    ) -> Result<WpskkaMessage<'static>, HigherError> {
        self.wpskka
            .process_password(password)
            .map_err(|error| HigherError::Wpskka(WpskkaError::Client(error)))
    }
}

impl<Wpskka: WpskkaHandlerTrait, Rvd: RvdHandlerTrait> HigherHandler<Wpskka, Rvd> {
    fn send_rvd(&mut self, rvd: RvdMessage) -> Result<ChanneledMessage<HigherOutput>, HigherError> {
        let data = rvd.to_bytes()?;

        match rvd {
            RvdMessage::FrameData(_) => {
                let cipher = self.wpskka.unreliable_cipher();
                let wpskka: TransportDataMessageUnreliable<'_> =
                    <Wpskka as WpskkaHandlerTrait>::wrap_unreliable(data, cipher)
                        .map_err(HigherSendError::from)?;
                Ok(self.send_wpskka(WpskkaMessage::TransportDataMessageUnreliable(wpskka))?)
            }
            _ => {
                let wpskka = self
                    .wpskka
                    .wrap_reliable(data)
                    .map_err(HigherSendError::from)?;
                Ok(self.send_wpskka(WpskkaMessage::TransportDataMessageReliable(wpskka))?)
            }
        }
    }

    fn send_wpskka(
        &mut self,
        message: WpskkaMessage<'_>,
    ) -> Result<ChanneledMessage<HigherOutput>, HigherSendError> {
        let output = HigherOutput(message.to_bytes()?);

        match message {
            // TransportDataMessageUnreliable is the only type of unreliable message
            WpskkaMessage::TransportDataMessageUnreliable(..) =>
                Ok(ChanneledMessage::Unreliable(output)),
            _ => Ok(ChanneledMessage::Reliable(output)),
        }
    }

    fn handle_internal(
        &mut self,
        wpskka_data: &[u8],
        output: &mut Vec<ChanneledMessage<HigherOutput>>,
    ) -> Result<Vec<InformEvent>, HigherError> {
        let mut events = Vec::new();

        let wpskka_message = WpskkaMessage::read(&mut Cursor::new(wpskka_data))?;
        let mut send_wpskka = Vec::new();
        let rvd_data = self
            .wpskka
            .handle(wpskka_message, &mut send_wpskka, &mut events)?;

        for message in send_wpskka {
            output.push(self.send_wpskka(message)?);
        }

        let rvd_data = match rvd_data {
            Some(data) => data,
            None => return Ok(events),
        };
        let rvd_message = RvdMessage::read(&mut Cursor::new(&rvd_data[..]))?;
        let mut send_rvd = Vec::new();
        self.rvd.handle(rvd_message, &mut send_rvd, &mut events)?;

        for message in send_rvd {
            output.push(self.send_rvd(message)?);
        }

        Ok(events)
    }

    fn send_internal<'a, M: Into<sealed::HigherMessage<'a>>>(
        &mut self,
        message: M,
    ) -> Result<ChanneledMessage<HigherOutput>, HigherError> {
        match message.into() {
            sealed::HigherMessage::Rvd(rvd) => self.send_rvd(rvd),
            sealed::HigherMessage::Wpskka(wpskka) => Ok(self.send_wpskka(wpskka)?),
        }
    }
}

impl HigherHandlerTrait for HigherHandlerHost {
    fn handle(
        &mut self,
        wpskka_data: &[u8],
        output: &mut Vec<ChanneledMessage<HigherOutput>>,
    ) -> Result<Vec<InformEvent>, HigherError> {
        self.handle_internal(wpskka_data, output)
    }

    fn send<'a, M: Into<sealed::HigherMessage<'a>>>(
        &mut self,
        message: M,
    ) -> Result<ChanneledMessage<HigherOutput>, HigherError> {
        self.send_internal(message)
    }
}

impl HigherHandlerTrait for HigherHandlerClient {
    fn handle(
        &mut self,
        wpskka_data: &[u8],
        output: &mut Vec<ChanneledMessage<HigherOutput>>,
    ) -> Result<Vec<InformEvent>, HigherError> {
        self.handle_internal(wpskka_data, output)
    }

    fn send<'a, M: Into<sealed::HigherMessage<'a>>>(
        &mut self,
        message: M,
    ) -> Result<ChanneledMessage<HigherOutput>, HigherError> {
        self.send_internal(message)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum HigherSendError {
    #[error("failed to encode message: {0}")]
    Encode(#[from] MessageComponentError),
    #[error("failed to encrypt message: {0}")]
    Cipher(#[from] CipherError),
}

#[derive(Debug, thiserror::Error)]
pub enum HigherError {
    #[error("failed to decode message: {0}")]
    Decode(#[from] MessageComponentError),
    #[error("WPSKKA handler error: {0}")]
    Wpskka(#[from] WpskkaError),
    #[error("RVD handler error: {0}")]
    Rvd(#[from] RvdError),
    #[error("send error: {0}")]
    SendError(#[from] HigherSendError),
}
