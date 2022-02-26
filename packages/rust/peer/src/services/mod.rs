pub mod helpers;
pub mod rvd;
pub mod sel_handler;
pub mod svsc_handler;
pub mod wpskka;

use crate::services::helpers::cipher_reliable_peer::CipherError;
use common::messages::{
    rvd::RvdMessage,
    sel::SelMessage,
    svsc::SvscMessage,
    wpskka::WpskkaMessage,
    Error as MessageComponentError,
    MessageComponent,
    ScreenViewMessage,
};

use self::{
    rvd::RvdHandler,
    sel_handler::SelHandler,
    svsc_handler::SvscError,
    wpskka::{WpskkaError, WpskkaHandler},
};
use crate::{
    io::NativeIoHandle,
    services::{
        rvd::{RvdClientInform, RvdError, RvdHostInform},
        sel_handler::SelError,
        svsc_handler::{SvscHandler, SvscInform},
        wpskka::{WpskkaClientInform, WpskkaHostInform},
    },
};
use native::api::NativeApiTemplate;
use std::io::Cursor;

pub struct ScreenViewHandler {
    sel: SelHandler,
    svsc: SvscHandler,
    wpskka: WpskkaHandler,
    rvd: RvdHandler,
    io_handle: NativeIoHandle,
}

impl ScreenViewHandler {
    pub fn send(&mut self, message: ScreenViewMessage) -> Result<(), SendError> {
        match message {
            ScreenViewMessage::RvdMessage(rvd) => self.send_rvd(rvd),
            ScreenViewMessage::WpskkaMessage(wpskka) => self.send_wpskka(wpskka),
            ScreenViewMessage::SvscMessage(svsc) => self.send_svsc(svsc, true),
            ScreenViewMessage::SelMessage(sel) => self.send_sel(sel),
        }
    }

    pub fn handle(&mut self, message: SelMessage) -> Result<Vec<InformEvent>, HandlerError> {
        let mut events = Vec::new();

        let svsc_data = self.sel.handle(message)?;
        let svsc_message = SvscMessage::read(&mut Cursor::new(&svsc_data[..]))?;
        let mut send_svsc = Vec::new();
        let wpskka_data = match self
            .svsc
            .handle(svsc_message, &mut send_svsc, &mut events)?
        {
            Some(data) => data,
            None => return Ok(events),
        };

        for message in send_svsc {
            self.send_svsc(message, true)?;
        }

        let wpskka_message = WpskkaMessage::read(&mut Cursor::new(&wpskka_data[..]))?;
        let mut send_wpskka = Vec::new();
        let rvd_data = self
            .wpskka
            .handle(wpskka_message, &mut send_wpskka, &mut events)?;
        for message in send_wpskka {
            self.send_wpskka(message)?;
        }

        let rvd_data = match rvd_data {
            Some(data) => data,
            None => return Ok(events),
        };
        let rvd_message = RvdMessage::read(&mut Cursor::new(&rvd_data[..]))?;
        let mut send_rvd = Vec::new();
        self.rvd.handle(rvd_message, &mut send_rvd, &mut events)?;
        for message in send_rvd {
            self.send_rvd(message)?;
        }

        Ok(events)
    }

    fn send_rvd(&mut self, rvd: RvdMessage) -> Result<(), SendError> {
        let data = rvd.to_bytes()?;

        match rvd {
            RvdMessage::FrameData(_) => {
                let cipher = self.wpskka.unreliable_cipher();
                let wpskka = WpskkaHandler::wrap_unreliable(data, cipher)?;
                self.send_wpskka(WpskkaMessage::TransportDataMessageUnreliable(wpskka))
            }
            _ => {
                let wpskka = self.wpskka.wrap_reliable(data)?;
                self.send_wpskka(WpskkaMessage::TransportDataMessageReliable(wpskka))
            }
        }
    }

    fn send_wpskka(&mut self, message: WpskkaMessage) -> Result<(), SendError> {
        let data = message.to_bytes()?;
        let svsc = SvscHandler::wrap(data);
        let reliable = !matches!(message, WpskkaMessage::TransportDataMessageUnreliable(_));
        self.send_svsc(SvscMessage::SessionDataSend(svsc), reliable)
    }

    fn send_svsc(&mut self, message: SvscMessage, reliable: bool) -> Result<(), SendError> {
        let data = message.to_bytes()?;

        let sel = if reliable {
            SelHandler::wrap_reliable(data)
        } else {
            let cipher = self.sel.unreliable_cipher();
            SelHandler::wrap_unreliable(data, *self.svsc.peer_id().unwrap(), cipher)?
        };

        self.send_sel(sel)
    }

    fn send_sel(&mut self, sel: SelMessage) -> Result<(), SendError> {
        let sent = match sel {
            SelMessage::TransportDataMessageReliable(_) => self.io_handle.send_reliable(sel),
            SelMessage::TransportDataPeerMessageUnreliable(_) =>
                self.io_handle.send_unreliable(sel),
            _ => unreachable!(),
        };

        if sent {
            Ok(())
        } else {
            Err(SendError::BrokenPipe)
        }
    }
}

pub enum InformEvent {
    SvscInform(SvscInform),
    RvdClientInform(RvdClientInform),
    RvdHostInform(RvdHostInform),
    WpskkaClientInform(WpskkaClientInform),
    WpskkaHostInform(WpskkaHostInform),
}

#[derive(thiserror::Error, Debug)]
pub enum SendError {
    #[error("failed to encode message: {0}")]
    Encode(#[from] MessageComponentError),
    #[error("failed to encrypt message: {0}")]
    Cipher(#[from] CipherError),
    #[error("failed to send SEL message to worker thread")]
    BrokenPipe,
}

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("failed to decode message: {0}")]
    Decode(#[from] MessageComponentError),
    #[error("SEL handler error: {0}")]
    Sel(#[from] SelError),
    #[error("SVSC handler error: {0}")]
    Svsc(#[from] SvscError),
    #[error("WPSKKA handler error: {0}")]
    Wpskka(#[from] WpskkaError),
    #[error("RVD handler error: {0}")]
    Rvd(#[from] RvdError),
    #[error("send error: {0}")]
    SendError(#[from] SendError),
}
