use crate::{
    native::{
        api::{MouseButton, MousePosition, NativeApiTemplate},
        NativeApi,
        NativeApiError,
    },
    services::{
        helpers::{clipboard_type_map::get_native_clipboard, rvd_common::*},
        SendError,
    },
};
use common::messages::rvd::{
    AccessMask,
    ButtonsMask,
    ClipboardNotification,
    DisplayChange,
    DisplayId,
    RvdMessage,
};

#[derive(Copy, Clone, Debug)]
pub enum HostState {
    Handshake,
    WaitingForDisplayChangeReceived,
    SendData,
}

#[derive(Default)]
struct HostPermissions {
    pub clipboard_readable: bool,
    pub clipboard_writable: bool,
}

pub struct RvdHostHandler {
    state: HostState,
    native: NativeApi,
    current_display_change: DisplayChange, // the displays we are sharing
}

impl RvdHostHandler {
    pub fn new(native: NativeApi) -> Self {
        Self {
            state: HostState::Handshake,
            native,
            current_display_change: Default::default(),
        }
    }

    fn permissions(&self) -> HostPermissions {
        return HostPermissions {
            clipboard_readable: self.current_display_change.clipboard_readable,
            clipboard_writable: self.current_display_change.clipboard_readable
                && self
                    .current_display_change
                    .display_information
                    .iter()
                    .any(|info| info.access.contains(AccessMask::CONTROLLABLE)),
        };
    }

    fn display_is_controllable(&self, display_id: DisplayId) -> Result<bool, RvdHostError> {
        Ok(!self
            .current_display_change
            .display_information
            .iter()
            .find(|info| info.display_id == display_id)
            .ok_or(RvdHostError::DisplayNotFound(display_id))?
            .access
            .contains(AccessMask::CONTROLLABLE))
    }

    pub fn handle(
        &mut self,
        msg: RvdMessage,
        mut write: &mut Vec<RvdMessage>,
    ) -> Result<(), RvdHostError> {
        match self.state {
            HostState::Handshake => match msg {
                RvdMessage::ProtocolVersionResponse(msg) => {
                    if !msg.ok {
                        return Err(RvdHostError::VersionBad);
                    }
                    self.state = HostState::WaitingForDisplayChangeReceived;
                    Ok(())
                }
                _ => Err(RvdHostError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
            HostState::WaitingForDisplayChangeReceived => match msg {
                // TODO edge: Wait for display change and we receive a message
                RvdMessage::DisplayChangeReceived(_) => {
                    self.state = HostState::SendData;
                    Ok(())
                }
                _ => Err(RvdHostError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
            HostState::SendData => match msg {
                RvdMessage::MouseInput(msg) => {
                    if self.display_is_controllable(msg.display_id)? {
                        return Err(RvdHostError::PermissionsError("mouse input".to_string()));
                    }
                    self.native.set_pointer_position(MousePosition {
                        x: msg.x_location as u32,
                        y: msg.y_location as u32,
                        monitor_id: msg.display_id,
                    })?;
                    // TODO macro?
                    self.native
                        .toggle_mouse(MouseButton::Left, msg.buttons.contains(ButtonsMask::LEFT))?;
                    self.native.toggle_mouse(
                        MouseButton::Center,
                        msg.buttons.contains(ButtonsMask::MIDDLE),
                    )?;
                    self.native.toggle_mouse(
                        MouseButton::Right,
                        msg.buttons.contains(ButtonsMask::RIGHT),
                    )?;
                    self.native.toggle_mouse(
                        MouseButton::ScrollUp,
                        msg.buttons.contains(ButtonsMask::SCROLL_UP),
                    )?;
                    self.native.toggle_mouse(
                        MouseButton::ScrollDown,
                        msg.buttons.contains(ButtonsMask::SCROLL_DOWN),
                    )?;
                    self.native.toggle_mouse(
                        MouseButton::ScrollLeft,
                        msg.buttons.contains(ButtonsMask::SCROLL_LEFT),
                    )?;
                    self.native.toggle_mouse(
                        MouseButton::ScrollRight,
                        msg.buttons.contains(ButtonsMask::SCROLL_RIGHT),
                    )?;
                    Ok(())
                }
                RvdMessage::KeyInput(msg) => {
                    if self
                        .current_display_change
                        .display_information
                        .iter()
                        .any(|info| info.access.contains(AccessMask::CONTROLLABLE))
                    {
                        return Err(RvdHostError::PermissionsError("key input".to_string()));
                    }
                    Ok(self.native.key_toggle(msg.key, msg.down)?)
                }
                RvdMessage::ClipboardRequest(msg) => {
                    clipboard_request_impl!(self, msg, write, RvdHostError)
                }
                RvdMessage::ClipboardNotification(msg) => {
                    clipboard_notificaiton_impl!(self, msg, RvdHostError)
                }
                _ => Err(RvdHostError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RvdHostError {
    #[error("client rejected version")]
    VersionBad,
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(Box<RvdMessage>, HostState),
    #[error("native error: {0}")]
    NativeError(#[from] NativeApiError),
    #[error("send_error")]
    WriteError(#[from] SendError),
    #[error("permission error: cannot {0}")]
    PermissionsError(String),
    #[error("display not found: id number {0}")]
    DisplayNotFound(DisplayId),
}
