use crate::{
    helpers::cipher_reliable_peer::CipherError,
    rvd::{RvdClientHandler, RvdError, RvdHandlerTrait, RvdHostHandler},
    wpskka::{WpskkaClientHandler, WpskkaError, WpskkaHandlerTrait, WpskkaHostHandler},
    InformEvent,
};
use common::messages::{
    rvd::RvdMessage,
    wpskka::{TransportDataMessageUnreliable, WpskkaMessage},
    Error as MessageComponentError,
    MessageComponent,
    ScreenViewMessage,
};
use std::io::Cursor;


pub type HigherOutput = (Vec<u8>, bool);

pub struct HigherHandler<Wpskka, Rvd> {
    wpskka: Wpskka,
    rvd: Rvd,
}

impl HigherHandler<WpskkaHostHandler, RvdHostHandler> {
    pub fn new(wpskka: WpskkaHostHandler, rvd: RvdHostHandler) -> Self {
        HigherHandler { wpskka, rvd }
    }
}

impl HigherHandler<WpskkaClientHandler, RvdClientHandler> {
    pub fn new(wpskka: WpskkaClientHandler, rvd: RvdClientHandler) -> Self {
        HigherHandler { wpskka, rvd }
    }
}

impl<Wpskka: WpskkaHandlerTrait, Rvd: RvdHandlerTrait> HigherHandler<Wpskka, Rvd> {
    pub fn send(&mut self, message: ScreenViewMessage) -> Result<HigherOutput, HigherSendError> {
        match message {
            ScreenViewMessage::RvdMessage(rvd) => self.send_rvd(rvd),
            ScreenViewMessage::WpskkaMessage(wpskka) => Ok(self.send_wpskka(wpskka)?),
            _ => panic!("You should only be sending RvdMessage or WpskkaMessage to HigherHandler"),
        }
    }

    fn send_rvd(&mut self, rvd: RvdMessage) -> Result<HigherOutput, HigherSendError> {
        let data = rvd.to_bytes()?;

        match rvd {
            RvdMessage::FrameData(_) => {
                let cipher = self.wpskka.unreliable_cipher();
                let wpskka: TransportDataMessageUnreliable =
                    <Wpskka as WpskkaHandlerTrait>::wrap_unreliable(data, cipher)?;
                Ok(self.send_wpskka(WpskkaMessage::TransportDataMessageUnreliable(wpskka))?)
            }
            _ => {
                let wpskka = self.wpskka.wrap_reliable(data)?;
                Ok(self.send_wpskka(WpskkaMessage::TransportDataMessageReliable(wpskka))?)
            }
        }
    }

    fn send_wpskka(&mut self, message: WpskkaMessage) -> Result<HigherOutput, HigherSendError> {
        // TransportDataMessageUnreliable is the only type of unreliable message
        let reliable = !matches!(message, WpskkaMessage::TransportDataMessageUnreliable(_));
        let bytes = MessageComponent::to_bytes(&message)?;
        Ok((bytes, reliable))
    }

    pub fn handle(
        &mut self,
        wpskka_data: &[u8],
        output: &mut Vec<HigherOutput>,
    ) -> Result<Vec<InformEvent>, HigherError> {
        let mut events = Vec::new();

        let wpskka_message = WpskkaMessage::read(&mut Cursor::new(&wpskka_data[..]))?;
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
