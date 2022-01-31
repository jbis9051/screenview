mod helpers;
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
    io::IoHandle,
    services::{
        rvd::{RvdError, RvdInform},
        sel_handler::SelError,
        svsc_handler::{SvscHandler, SvscInform},
        wpskka::{WpskkaClientInform, WpskkaHostInform},
    },
};
use std::io::Cursor;

pub struct ScreenViewHandler {
    sel: SelHandler,
    svsc: SvscHandler,
    wpskka: WpskkaHandler,
    rvd: RvdHandler,
    io_handle: IoHandle,
}

impl ScreenViewHandler {
    pub fn send(&mut self, message: ScreenViewMessage) -> Result<(), SendError> {
        let sel_handler = &self.sel;
        let svsc_handler = &self.svsc;
        let wpskka_handler = &mut self.wpskka;
        let io_handle = &self.io_handle;

        match message {
            ScreenViewMessage::RvdMessage(rvd) =>
                Self::send_rvd(io_handle, sel_handler, svsc_handler, wpskka_handler, rvd),
            ScreenViewMessage::WpskkaMessage(wpskka) =>
                Self::send_wpskka(io_handle, sel_handler, svsc_handler, wpskka),
            ScreenViewMessage::SvscMessage(svsc) =>
                Self::send_svsc(io_handle, sel_handler, svsc_handler, svsc, true),
            ScreenViewMessage::SelMessage(sel) => {
                let reliable = matches!(sel, SelMessage::TransportDataMessageReliable(_));
                Self::send_sel(io_handle, sel, reliable)
            }
        }
    }

    pub fn handle(&mut self, message: SelMessage) -> Result<Vec<InformEvent>, HandlerError> {
        let sel = &mut self.sel;
        let svsc = &mut self.svsc;
        let wpskka = &mut self.wpskka;
        let rvd = &mut self.rvd;
        let io_handle = &self.io_handle;
        let mut events = Vec::new();

        let svsc_data = sel.handle(message)?;
        let svsc_message = SvscMessage::read(&mut Cursor::new(&svsc_data[..]))?;
        let wpskka_data = match svsc.handle(svsc_message, &mut events)? {
            Some(data) => data,
            None => return Ok(events),
        };
        let wpskka_message = WpskkaMessage::read(&mut Cursor::new(&wpskka_data[..]))?;
        let rvd_data = wpskka.handle(
            wpskka_message,
            |msg| Self::send_wpskka(io_handle, sel, svsc, msg),
            &mut events,
        )?;
        let rvd_data = match rvd_data {
            Some(data) => data,
            None => return Ok(events),
        };
        let rvd_message = RvdMessage::read(&mut Cursor::new(&rvd_data[..]))?;
        rvd.handle(
            rvd_message,
            |msg| Self::send_rvd(io_handle, sel, svsc, wpskka, msg),
            &mut events,
        )?;

        Ok(events)
    }

    fn send_rvd(
        io_handle: &IoHandle,
        sel_handler: &SelHandler,
        svsc_handler: &SvscHandler,
        wpskka_handler: &mut WpskkaHandler,
        rvd: RvdMessage,
    ) -> Result<(), SendError> {
        let data = rvd.to_bytes()?;

        match rvd {
            RvdMessage::FrameData(_) => {
                let cipher = &**wpskka_handler.unreliable_cipher();
                let wpskka = WpskkaHandler::wrap_unreliable(data, cipher)?;
                Self::send_wpskka(
                    io_handle,
                    sel_handler,
                    svsc_handler,
                    WpskkaMessage::TransportDataMessageUnreliable(wpskka),
                )
            }
            _ => {
                let wpskka = wpskka_handler.wrap_reliable(data)?;
                Self::send_wpskka(
                    io_handle,
                    sel_handler,
                    svsc_handler,
                    WpskkaMessage::TransportDataMessageReliable(wpskka),
                )
            }
        }
    }

    fn send_wpskka(
        io_handle: &IoHandle,
        sel_handler: &SelHandler,
        svsc_handler: &SvscHandler,
        wpskka: WpskkaMessage,
    ) -> Result<(), SendError> {
        let data = wpskka.to_bytes()?;
        let svsc = SvscHandler::wrap(data);
        let reliable = !matches!(wpskka, WpskkaMessage::TransportDataMessageUnreliable(_));
        Self::send_svsc(
            io_handle,
            sel_handler,
            svsc_handler,
            SvscMessage::SessionDataSend(svsc),
            reliable,
        )
    }

    fn send_svsc(
        io_handle: &IoHandle,
        sel_handler: &SelHandler,
        svsc_handler: &SvscHandler,
        svsc: SvscMessage,
        reliable: bool,
    ) -> Result<(), SendError> {
        let data = svsc.to_bytes()?;

        let sel = if reliable {
            SelMessage::TransportDataMessageReliable(SelHandler::wrap_reliable(data))
        } else {
            let cipher = &**sel_handler.unreliable_cipher();
            SelMessage::TransportDataPeerMessageUnreliable(SelHandler::wrap_unreliable(
                data,
                svsc_handler.peer_id(),
                cipher,
            )?)
        };

        Self::send_sel(io_handle, sel, reliable)
    }

    fn send_sel(io_handle: &IoHandle, sel: SelMessage, reliable: bool) -> Result<(), SendError> {
        let sent = if reliable {
            io_handle.send_reliable(sel)
        } else {
            io_handle.send_unreliable(sel)
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
    RvdInform(RvdInform),
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
}
