use crate::services::helpers::clipboard_type_map::get_native_clipboard;
use common::messages::rvd::{
    ButtonsMask, ClipboardNotification, DisplayChange, DisplayChangeReceived, DisplayInformation,
    ProtocolVersionResponse, RvdMessage,
};
use common::messages::ScreenViewMessage;
use native::api::{MouseButton, MousePosition, NativeApiTemplate};
use std::sync::mpsc::{SendError, Sender};

#[derive(Copy, Clone, Debug)]
enum State {
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

    pub fn handle(
        &mut self,
        msg: RvdMessage,
        send: Sender<ScreenViewMessage>,
    ) -> Result<(), RvdClientError<T>> {
        match self.state {
            State::Handshake => match msg {
                RvdMessage::ProtocolVersion(msg) => {
                    // TODO check version
                    send.send(ScreenViewMessage::RvdMessage(
                        RvdMessage::ProtocolVersionResponse(ProtocolVersionResponse { ok: true }),
                    ))
                    .map_err(RvdClientError::SendError)?;
                    self.state = State::Data;
                    Ok(())
                }
                _ => Err(RvdClientError::WrongMessageForState(msg, self.state)),
            },
            State::Data => match msg {
                RvdMessage::DisplayChange(msg) => {
                    self.current_display_change = msg;
                    send.send(ScreenViewMessage::RvdMessage(
                        RvdMessage::DisplayChangeReceived(DisplayChangeReceived {}),
                    ))
                    .map_err(RvdClientError::SendError)?;
                    Ok(())
                }
                RvdMessage::ClipboardRequest(msg) => {
                    if !self.permissions.clipboard_readable {
                        return Err(RvdClientError::PermissionsError(
                            "read clipboard".to_owned(),
                        ));
                    }
                    let clip_type = get_native_clipboard(&msg.info.clipboard_type);
                    let content = self.native.clipboard_content(&clip_type).ok();
                    send.send(ScreenViewMessage::RvdMessage(
                        RvdMessage::ClipboardNotification(ClipboardNotification {
                            info: msg.info.clone(),
                            content: if msg.info.content_request {
                                content
                            } else {
                                None
                            },
                        }),
                    ))
                    .map_err(RvdClientError::SendError)?;
                    Ok(())
                }
                RvdMessage::ClipboardNotification(msg) => {
                    if !self.permissions.clipboard_writable {
                        return Err(RvdClientError::PermissionsError(
                            "write clipboard".to_owned(),
                        ));
                    }
                    let clip_type = get_native_clipboard(&msg.info.clipboard_type);
                    if !msg.info.content_request {
                        // TODO handle
                        return Ok(());
                    }
                    self.native
                        .set_clipboard_content(&clip_type, &msg.content.unwrap())
                        .map_err(RvdClientError::NativeError)?;
                    Ok(())
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
    #[error("send_error")]
    SendError(SendError<ScreenViewMessage>),
    #[error("permission error: cannot {0}")]
    PermissionsError(String),
}
