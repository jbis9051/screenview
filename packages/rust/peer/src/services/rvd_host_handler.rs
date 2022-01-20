use crate::services::helpers::clipboard_type_map::get_native_clipboard;
use crate::services::helpers::rvd_macro::*;
use common::messages::rvd::{ButtonsMask, ClipboardNotification, RvdMessage};
use common::messages::ScreenViewMessage;
use native::api::{MouseButton, MousePosition, NativeApiTemplate};
use std::sync::mpsc::{SendError, Sender};

#[derive(Copy, Clone, Debug)]
pub enum State {
    Handshake,
    WaitingForDisplayChangeReceived,
    SendData,
}

#[derive(Default)]
struct Permissions {
    pub clipboard_readable: bool,
    pub clipboard_writable: bool,
}

pub struct RvdHostHandler<T: NativeApiTemplate> {
    state: State,
    native: T,
    permissions: Permissions,
}

impl<T: NativeApiTemplate> RvdHostHandler<T> {
    pub fn new(native: T) -> Self {
        Self {
            state: State::Handshake,
            native,
            permissions: Default::default(),
        }
    }

    pub fn handle(
        &mut self,
        msg: RvdMessage,
        send: Sender<ScreenViewMessage>,
    ) -> Result<(), RvdHostError<T>> {
        match self.state {
            State::Handshake => match msg {
                RvdMessage::ProtocolVersionResponse(msg) => {
                    if !msg.ok {
                        return Err(RvdHostError::VersionBad);
                    }
                    self.state = State::WaitingForDisplayChangeReceived;
                    Ok(())
                }
                _ => Err(RvdHostError::WrongMessageForState(msg, self.state)),
            },
            State::WaitingForDisplayChangeReceived => match msg {
                // TODO edge: Wait for display change and we receive a message
                RvdMessage::DisplayChangeReceived(_) => {
                    self.state = State::SendData;
                    Ok(())
                }
                _ => Err(RvdHostError::WrongMessageForState(msg, self.state)),
            },
            State::SendData => match msg {
                RvdMessage::MouseInput(msg) => {
                    self.native
                        .set_pointer_position(MousePosition {
                            x: msg.x_location as u32,
                            y: msg.y_location as u32,
                            monitor_id: msg.display_id,
                        })
                        .map_err(RvdHostError::NativeError)?;
                    // TODO macro?
                    self.native
                        .toggle_mouse(MouseButton::Left, msg.buttons.contains(ButtonsMask::LEFT))
                        .map_err(RvdHostError::NativeError)?;
                    self.native
                        .toggle_mouse(
                            MouseButton::Center,
                            msg.buttons.contains(ButtonsMask::MIDDLE),
                        )
                        .map_err(RvdHostError::NativeError)?;
                    self.native
                        .toggle_mouse(MouseButton::Right, msg.buttons.contains(ButtonsMask::RIGHT))
                        .map_err(RvdHostError::NativeError)?;
                    self.native
                        .toggle_mouse(
                            MouseButton::ScrollUp,
                            msg.buttons.contains(ButtonsMask::SCROLL_UP),
                        )
                        .map_err(RvdHostError::NativeError)?;
                    self.native
                        .toggle_mouse(
                            MouseButton::ScrollDown,
                            msg.buttons.contains(ButtonsMask::SCROLL_DOWN),
                        )
                        .map_err(RvdHostError::NativeError)?;
                    self.native
                        .toggle_mouse(
                            MouseButton::ScrollLeft,
                            msg.buttons.contains(ButtonsMask::SCROLL_LEFT),
                        )
                        .map_err(RvdHostError::NativeError)?;
                    self.native
                        .toggle_mouse(
                            MouseButton::ScrollRight,
                            msg.buttons.contains(ButtonsMask::SCROLL_RIGHT),
                        )
                        .map_err(RvdHostError::NativeError)?;
                    Ok(())
                }
                RvdMessage::KeyInput(msg) => Ok(self
                    .native
                    .key_toggle(msg.key, msg.down)
                    .map_err(RvdHostError::NativeError)?),
                RvdMessage::ClipboardRequest(msg) => {
                    clipboard_request_impl!(self, msg, send, RvdHostError<T>)
                }
                RvdMessage::ClipboardNotification(msg) => {
                    clipboard_notificaiton_impl!(self, msg, RvdHostError<T>)
                }
                _ => Err(RvdHostError::WrongMessageForState(msg, self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RvdHostError<T: NativeApiTemplate> {
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
