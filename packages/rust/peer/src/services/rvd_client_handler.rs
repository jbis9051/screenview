use crate::services::helpers::clipboard_type_map::get_native_clipboard;
use crate::services::helpers::rvd_common::*;
use crate::services::InformEvent;
use common::constants::SVSC_VERSION;
use common::messages::rvd::{
    ClipboardNotification, DisplayChange, DisplayChangeReceived, MouseLocation,
    ProtocolVersionResponse, RvdMessage,
};
use common::messages::ScreenViewMessage;
use native::api::NativeApiTemplate;
use std::sync::mpsc::{SendError, Sender};

#[derive(Copy, Clone, Debug)]
pub enum State {
    Handshake,
    Data,
}

#[derive(Default)]
struct Permissions {
    pub clipboard_readable: bool,
    pub clipboard_writable: bool,
}

pub struct RvdClientHandler<T: NativeApiTemplate> {
    state: State,
    native: T,
    permissions: Permissions,
    current_display_change: DisplayChange,
}

impl<T: NativeApiTemplate> RvdClientHandler<T> {
    pub fn new(native: T) -> Self {
        Self {
            state: State::Handshake,
            native,
            permissions: Default::default(),
            current_display_change: Default::default(),
        }
    }

    fn permissions(&self) -> &Permissions {
        return &self.permissions;
    }

    pub fn handle(
        &mut self,
        msg: RvdMessage,
        write: Sender<ScreenViewMessage>,
        events: Sender<InformEvent>,
    ) -> Result<(), RvdClientError<T>> {
        match self.state {
            State::Handshake => match msg {
                RvdMessage::ProtocolVersion(msg) => {
                    let ok = msg.version == SVSC_VERSION;
                    write
                        .send(ScreenViewMessage::RvdMessage(
                            RvdMessage::ProtocolVersionResponse(ProtocolVersionResponse { ok }),
                        ))
                        .map_err(RvdClientError::WriteError)?;
                    self.state = State::Data;
                    if ok {
                        Ok(())
                    } else {
                        Err(RvdClientError::VersionBad)
                    }
                }
                _ => Err(RvdClientError::WrongMessageForState(msg, self.state)),
            },
            State::Data => match msg {
                RvdMessage::FrameData(_) => {
                    todo!()
                }
                RvdMessage::DisplayChange(msg) => {
                    self.current_display_change = msg;
                    write
                        .send(ScreenViewMessage::RvdMessage(
                            RvdMessage::DisplayChangeReceived(DisplayChangeReceived {}),
                        ))
                        .map_err(RvdClientError::WriteError)?;
                    Ok(())
                }
                RvdMessage::MouseLocation(msg) => events
                    .send(InformEvent::RvdInform(RvdInform::MouseLocation(msg)))
                    .map_err(RvdClientError::InformError),
                RvdMessage::ClipboardRequest(msg) => {
                    clipboard_request_impl!(self, msg, write, RvdClientError<T>)
                }
                RvdMessage::ClipboardNotification(msg) => {
                    clipboard_notificaiton_impl!(self, msg, RvdClientError<T>)
                }
                _ => Err(RvdClientError::WrongMessageForState(msg, self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RvdClientError<T: NativeApiTemplate> {
    #[error("client rejected version")]
    VersionBad,
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(RvdMessage, State),
    #[error("native error: {0}")]
    NativeError(T::Error),
    #[error("write error")]
    WriteError(SendError<ScreenViewMessage>),
    #[error("inform error")]
    InformError(SendError<InformEvent>),
    #[error("permission error: cannot {0}")]
    PermissionsError(String),
}

pub enum RvdInform {
    MouseLocation(MouseLocation),
}
