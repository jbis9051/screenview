use crate::{
    capture::FrameUpdate,
    debug,
    rvd::{
        PermissionError::{ClipboardRead, ClipboardWrite, MouseInput},
        RvdError,
        RvdHandlerTrait,
    },
    InformEvent,
};
use common::{
    constants::RVD_VERSION,
    messages::rvd::{
        AccessMask,
        ButtonsMask,
        ClipboardType,
        DisplayChange,
        DisplayId,
        DisplayInformation,
        FrameData,
        KeyInput,
        ProtocolVersion,
        RvdMessage,
    },
};
use native::api::{MonitorId, WindowId};
use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Monitor(MonitorId),
    Window(WindowId),
}

impl Display {
    pub fn id(&self) -> u32 {
        match *self {
            Self::Monitor(id) => id,
            Self::Window(id) => id,
        }
    }

    pub fn display_type(&self) -> DisplayType {
        match self {
            Self::Monitor(_) => DisplayType::Monitor,
            Self::Window(_) => DisplayType::Window,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum DisplayType {
    Monitor,
    Window,
}

struct SharedDisplay {
    needs_flush: bool,
    frame_number: u32,
    display: RvdDisplay,
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
    shared_displays: Vec<Option<SharedDisplay>>,
}

impl Default for RvdHostHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl RvdHostHandler {
    pub fn new() -> Self {
        Self {
            state: HostState::Handshake,
            clipboard_readable: false,
            controllable: false,
            shared_displays: Vec::new(),
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

    pub fn share_display(&mut self, display: RvdDisplay) -> ShareDisplayResult {
        let mut slot_info = None;

        for (index, slot) in self.shared_displays.iter_mut().enumerate() {
            match slot {
                Some(shared) => {
                    if shared.display.native_id == display.native_id
                        && shared.display.display_type == display.display_type
                    {
                        shared.needs_flush = shared.display.width != display.width
                            || shared.display.height != display.height;
                        shared.display = display;
                        return ShareDisplayResult::AlreadySharing(index as DisplayId);
                    }
                }
                None =>
                    if slot_info.is_none() {
                        slot_info = Some((index, slot));
                    },
            }
        }

        let (display_id, slot) = match slot_info {
            Some((position, slot)) => (position as DisplayId, slot),
            None => {
                let num_shared_displays = self.shared_displays.len();

                if num_shared_displays >= 255 {
                    return ShareDisplayResult::IdLimitReached;
                }

                self.shared_displays.push(None);
                (
                    num_shared_displays as DisplayId,
                    self.shared_displays.last_mut().unwrap(),
                )
            }
        };

        *slot = Some(SharedDisplay {
            needs_flush: true,
            frame_number: 0,
            display,
        });
        ShareDisplayResult::NewlyShared(display_id)
    }

    /// Add displays using share_display
    /// This shares all displays in share_buffer. Updates to displays are handled properly.
    /// share_buffer is cleared (set to Vec::default() aka Vec::new() aka [])
    pub fn display_update(&mut self) -> RvdMessage {
        self.state = HostState::WaitingForDisplayChangeReceived;

        let mut access = AccessMask::empty();
        if self.controllable {
            access.insert(AccessMask::CONTROLLABLE);
        }

        let display_information = self
            .shared_displays
            .iter_mut()
            .enumerate()
            .flat_map(|(index, shared)| shared.as_mut().map(|shared| (index as DisplayId, shared)))
            .map(|(display_id, shared)| {
                access.set(AccessMask::FLUSH, shared.needs_flush);
                shared.needs_flush = false;

                DisplayInformation {
                    display_id,
                    width: shared.display.width,
                    height: shared.display.height,
                    cell_width: 0,
                    cell_height: 0,
                    access,
                    name: shared.display.name.clone(),
                }
            })
            .collect::<Vec<_>>();

        RvdMessage::DisplayChange(DisplayChange {
            clipboard_readable: self.clipboard_readable && self.controllable,
            display_information,
        })
    }

    pub fn frame_update<'a>(
        &mut self,
        fragments: FrameUpdate<'a>,
    ) -> impl Iterator<Item = RvdMessage> + 'a {
        let display_id = fragments.display_id;
        let shared_display = self
            .shared_displays
            .get_mut(display_id as usize)
            .and_then(Option::as_mut)
            .expect("invalid or stale display id");

        let frame_number = shared_display.frame_number;
        shared_display.frame_number = frame_number
            .checked_add(1)
            .expect("frame number overflowed");

        fragments.map(move |fragment| {
            RvdMessage::FrameData(FrameData {
                frame_number,
                display_id,
                cell_number: fragment.cell_number,
                data: fragment.data,
            })
        })
    }

    pub fn _handle(
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
                _ => Err(RvdHostError::WrongMessageForState(debug(&msg), self.state)),
            },
            HostState::WaitingForDisplayChangeReceived => match msg {
                // TODO edge: Wait for display change and we receive a message
                RvdMessage::DisplayChangeReceived(_) => {
                    self.state = HostState::SendData;
                    Ok(())
                }
                _ => Err(RvdHostError::WrongMessageForState(debug(&msg), self.state)),
            },
            HostState::SendData => match msg {
                RvdMessage::MouseInput(msg) => {
                    if !self.controllable {
                        return Err(RvdHostError::PermissionsError(MouseInput));
                    }

                    let (_, shared) = self
                        .shared_displays
                        .iter()
                        .enumerate()
                        .flat_map(|(index, opt)| {
                            opt.as_ref().map(|shared| (index as DisplayId, shared))
                        })
                        .find(|&(id, _)| id == msg.display_id)
                        .ok_or(RvdHostError::DisplayNotFound(msg.display_id))?;

                    events.push(InformEvent::RvdHostInform(RvdHostInform::MouseInput(
                        MouseInputEvent {
                            x_location: msg.x_location,
                            y_location: msg.y_location,
                            button_delta: msg.buttons_delta,
                            button_state: msg.buttons_state,
                            display_type: shared.display.display_type,
                            native_id: shared.display.native_id,
                        },
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
                    if let Some(content) = msg.content {
                        // only emit when theres content
                        events.push(InformEvent::RvdHostInform(
                            RvdHostInform::ClipboardNotification(content, msg.info.clipboard_type),
                        ));
                    }
                    Ok(())
                }
                _ => Err(RvdHostError::WrongMessageForState(debug(&msg), self.state)),
            },
        }
    }
}

impl RvdHandlerTrait for RvdHostHandler {
    fn handle(
        &mut self,
        msg: RvdMessage,
        _write: &mut Vec<RvdMessage>,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdError> {
        Ok(self._handle(msg, events)?)
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
    #[error("invalid message {0} for state {1:?}")]
    WrongMessageForState(String, HostState),
    #[error("permission error: cannot {0:?}")]
    PermissionsError(PermissionError),
    #[error("display not found: id number {0}")]
    DisplayNotFound(DisplayId),
}

pub struct MouseInputEvent {
    pub x_location: u16,
    pub y_location: u16,
    pub button_delta: ButtonsMask,
    pub button_state: ButtonsMask,
    pub display_type: DisplayType,
    pub native_id: u32,
}

pub enum RvdHostInform {
    VersionBad,

    MouseInput(MouseInputEvent),
    KeyboardInput(KeyInput),

    ClipboardRequest(bool, ClipboardType),
    ClipboardNotification(Vec<u8>, ClipboardType),
}

pub enum ShareDisplayResult {
    NewlyShared(DisplayId),
    AlreadySharing(DisplayId),
    IdLimitReached,
}
