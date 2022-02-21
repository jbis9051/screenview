use crate::{
    native::{api::NativeApiTemplate, NativeApi, NativeApiError},
    services::{
        helpers::{clipboard_type_map::get_native_clipboard, rvd_common::*},
        InformEvent,
        SendError,
    },
};
use common::{
    constants::SVSC_VERSION,
    messages::rvd::{
        ClipboardNotification,
        DisplayChange,
        DisplayChangeReceived,
        MouseLocation,
        ProtocolVersionResponse,
        RvdMessage,
    },
};

#[derive(Copy, Clone, Debug)]
pub enum ClientState {
    Handshake,
    Data,
}

#[derive(Default)]
struct ClientPermissions {
    pub clipboard_readable: bool,
    pub clipboard_writable: bool,
}

pub struct RvdClientHandler {
    state: ClientState,
    native: NativeApi,
    permissions: ClientPermissions,
    current_display_change: DisplayChange,
}

impl RvdClientHandler {
    pub fn new(native: NativeApi) -> Self {
        Self {
            state: ClientState::Handshake,
            native,
            permissions: Default::default(),
            current_display_change: Default::default(),
        }
    }

    fn permissions(&self) -> &ClientPermissions {
        &self.permissions
    }

    pub fn handle(
        &mut self,
        msg: RvdMessage,
        write: &mut Vec<RvdMessage>,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdClientError> {
        match self.state {
            ClientState::Handshake => match msg {
                RvdMessage::ProtocolVersion(msg) => {
                    let ok = msg.version == SVSC_VERSION;
                    write.push(RvdMessage::ProtocolVersionResponse(
                        ProtocolVersionResponse { ok },
                    ));
                    self.state = ClientState::Data;
                    if ok {
                        Ok(())
                    } else {
                        Err(RvdClientError::VersionBad)
                    }
                }
                _ => Err(RvdClientError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
            ClientState::Data => match msg {
                RvdMessage::FrameData(_) => {
                    todo!()
                }
                RvdMessage::DisplayChange(msg) => {
                    self.current_display_change = msg;
                    write.push(RvdMessage::DisplayChangeReceived(DisplayChangeReceived {}));
                    Ok(())
                }
                RvdMessage::MouseLocation(msg) => {
                    events.push(InformEvent::RvdInform(RvdInform::MouseLocation(msg)));
                    Ok(())
                }
                RvdMessage::ClipboardRequest(msg) => {
                    clipboard_request_impl!(self, msg, write, RvdClientError)
                }
                RvdMessage::ClipboardNotification(msg) => {
                    clipboard_notificaiton_impl!(self, msg, RvdClientError)
                }
                _ => Err(RvdClientError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RvdClientError {
    #[error("client rejected version")]
    VersionBad,
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(Box<RvdMessage>, ClientState),
    #[error("native error: {0}")]
    NativeError(#[from] NativeApiError),
    #[error("permission error: cannot {0}")]
    PermissionsError(String),
}

pub enum RvdInform {
    MouseLocation(MouseLocation),
}
