use crate::services::{
    rvd::PermissionError::{ClipboardRead, ClipboardWrite, MouseInput},
    InformEvent,
    SendError,
};
use common::{
    constants::RVD_VERSION,
    messages::rvd::{
        AccessMask,
        ButtonsMask,
        ClipboardMeta,
        ClipboardNotification,
        ClipboardType,
        DisplayChange,
        DisplayId,
        DisplayInformation,
        KeyInput,
        ProtocolVersion,
        RvdMessage,
    },
};
use native::api::{MouseButton, MousePosition, NativeApiTemplate};
use std::fmt::Debug;

#[derive(PartialEq)]
pub enum DisplayType {
    Monitor,
    Window,
}

struct DisplayMap {
    native_id: u32,
    display_type: DisplayType,
    information: DisplayInformation,
}

pub struct RvdDisplay {
    pub native_id: u32,
    pub name: String,
    pub display_type: DisplayType,
    pub width: u16,
    pub height: u16,
}

#[derive(Copy, Clone, Debug)]
pub enum HostState {
    Handshake,
    WaitingForDisplayChangeReceived,
    SendData,
}

// While the spec allows for each individual display to be controllable or not we only support all are controllable or none are
pub struct RvdHostHandler {
    state: HostState,
    clipboard_readable: bool,
    controllable: bool,
    shared_displays: Vec<DisplayMap>,
    share_buffer: Vec<RvdDisplay>,
}

impl RvdHostHandler {
    pub fn new() -> Self {
        Self {
            state: HostState::Handshake,
            clipboard_readable: false,
            controllable: false,
            shared_displays: Vec::new(),
            share_buffer: Vec::new(),
        }
    }

    pub fn protocol_version() -> RvdMessage {
        RvdMessage::ProtocolVersion(ProtocolVersion {
            version: RVD_VERSION.to_string(),
        })
    }

    pub fn set_clipboard_readable(&mut self, is_readable: bool) {
        if self.clipboard_readable == is_readable {
            return;
        }
        self.clipboard_readable = is_readable;
        // TODO send update
    }

    pub fn set_controllable(&mut self, is_controllable: bool) {
        if self.controllable == is_controllable {
            return;
        }
        self.controllable = is_controllable;
        // TODO send update
    }

    pub fn share_display(&mut self, display: RvdDisplay) {
        self.share_buffer.push(display);
    }

    /// Add displays using share_display
    /// This shares all displays in share_display. Updates to displays are handled properly.
    pub fn display_update(&mut self) -> RvdMessage {
        self.state = HostState::WaitingForDisplayChangeReceived;
        let mut access = AccessMask::FLUSH;
        if self.controllable {
            access |= AccessMask::CONTROLLABLE;
        }
        self.share_buffer
            .dedup_by(|a, b| a.display_type == b.display_type && a.native_id == b.native_id);
        self.shared_displays = std::mem::take(&mut self.share_buffer)
            .into_iter()
            .map(|d| {
                let mut access = access;

                if self
                    .shared_displays
                    .iter()
                    .find(|a| {
                        a.display_type == d.display_type
                            && a.native_id == d.native_id
                            && a.information.width == d.width
                            && a.information.height == d.height
                    })
                    .is_some()
                {
                    access.remove(AccessMask::FLUSH)
                }

                DisplayMap {
                    native_id: d.native_id,
                    display_type: d.display_type,
                    information: DisplayInformation {
                        display_id: 0,
                        width: d.width,
                        height: d.height,
                        cell_width: 0,  // TODO
                        cell_height: 0, // TODO
                        access,
                        name: d.name,
                    },
                }
            })
            .collect();

        RvdMessage::DisplayChange(DisplayChange {
            clipboard_readable: self.clipboard_readable && self.controllable,
            display_information: self
                .shared_displays
                .iter()
                .map(|d| d.information.clone())
                .collect(),
        })
    }

    pub fn handle(
        &mut self,
        msg: RvdMessage,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdHostError> {
        match self.state {
            HostState::Handshake => match msg {
                RvdMessage::ProtocolVersionResponse(msg) => {
                    if !msg.ok {
                        events.push(InformEvent::RvdHostInform(RvdHostInform::VersionBad));
                        return Ok(());
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
                    if !self.controllable {
                        return Err(RvdHostError::PermissionsError(MouseInput));
                    }
                    events.push(InformEvent::RvdHostInform(RvdHostInform::MouseInput(
                        MousePosition {
                            x: msg.x_location as u32,
                            y: msg.y_location as u32,
                            monitor_id: msg.display_id,
                        },
                        msg.buttons,
                    )));
                    Ok(())
                }
                RvdMessage::KeyInput(msg) => {
                    if !self.controllable {
                        return Err(RvdHostError::PermissionsError(PermissionError::KeyInput));
                    }
                    events.push(InformEvent::RvdHostInform(RvdHostInform::KeyboardInput(
                        KeyInput {
                            down: msg.down,
                            key: msg.key,
                        },
                    )));
                    Ok(())
                }
                RvdMessage::ClipboardRequest(msg) => {
                    if !self.clipboard_readable {
                        return Err(RvdHostError::PermissionsError(ClipboardRead));
                    }
                    events.push(InformEvent::RvdHostInform(RvdHostInform::ClipboardRequest(
                        msg.info.content_request,
                        msg.info.clipboard_type,
                    )));
                    Ok(())
                }
                RvdMessage::ClipboardNotification(msg) => {
                    if !(self.clipboard_readable && self.controllable) {
                        return Err(RvdHostError::PermissionsError(ClipboardWrite));
                    }
                    events.push(InformEvent::RvdHostInform(
                        RvdHostInform::ClipboardNotification(msg.content, msg.info.clipboard_type),
                    ));
                    Ok(())
                }
                _ => Err(RvdHostError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
        }
    }
}

#[derive(Debug)]
pub enum PermissionError {
    MouseInput,
    KeyInput,
    ClipboardRead,
    ClipboardWrite,
}

#[derive(Debug, thiserror::Error)]
pub enum RvdHostError {
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(Box<RvdMessage>, HostState),
    #[error("permission error: cannot {0:?}")]
    PermissionsError(PermissionError),
    #[error("display not found: id number {0}")]
    DisplayNotFound(DisplayId),
}

pub enum RvdHostInform {
    VersionBad,

    MouseInput(MousePosition, ButtonsMask),
    KeyboardInput(KeyInput),

    ClipboardRequest(bool, ClipboardType),
    ClipboardNotification(Option<Vec<u8>>, ClipboardType),
}
