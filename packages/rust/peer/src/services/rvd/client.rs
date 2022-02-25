use crate::services::{
    helpers::{clipboard_type_map::get_native_clipboard, rvd_common::*},
    InformEvent,
    SendError,
};
use common::{
    constants::RVD_VERSION,
    messages::rvd::{
        ClipboardNotification,
        DisplayChange,
        DisplayChangeReceived,
        MouseLocation,
        ProtocolVersionResponse,
        RvdMessage,
    },
};
use native::api::NativeApiTemplate;

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

pub struct RvdClientHandler<T: NativeApiTemplate> {
    state: ClientState,
    native: T,
    permissions: ClientPermissions,
    current_display_change: DisplayChange,
}

impl<T: NativeApiTemplate> RvdClientHandler<T> {
    pub fn new(native: T) -> Self {
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
    ) -> Result<(), RvdClientError<T>> {
        match self.state {
            ClientState::Handshake => match msg {
                RvdMessage::ProtocolVersion(msg) => {
                    let ok = msg.version == RVD_VERSION;
                    write.push(RvdMessage::ProtocolVersionResponse(
                        ProtocolVersionResponse { ok },
                    ));
                    if ok {
                        self.state = ClientState::Data;
                    } else {
                        events.push(InformEvent::RvdClientInform(RvdClientInform::VersionBad));
                    }
                    Ok(())
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
                    events.push(InformEvent::RvdClientInform(
                        RvdClientInform::MouseLocation(msg),
                    ));
                    Ok(())
                }
                RvdMessage::ClipboardRequest(msg) => {
                    clipboard_request_impl!(self, msg, write, RvdClientError<T>)
                }
                RvdMessage::ClipboardNotification(msg) => {
                    clipboard_notificaiton_impl!(self, msg, RvdClientError<T>)
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
pub enum RvdClientError<T: NativeApiTemplate> {
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(Box<RvdMessage>, ClientState),
    #[error("native error: {0:?}")]
    NativeError(T::Error),
    #[error("permission error: cannot {0}")]
    PermissionsError(String),
}

pub enum RvdClientInform {
    VersionBad,

    MouseLocation(MouseLocation),
}
