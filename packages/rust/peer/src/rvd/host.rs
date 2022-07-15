use crate::{
    debug,
    rvd::{
        PermissionError::{
            ClipboardRead,
            ClipboardWrite,
            KeyInput as KeyInputPermission,
            MouseInput,
        },
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
        DisplayId,
        DisplayShare,
        DisplayUnshare,
        KeyInput,
        PermissionMask,
        PermissionsUpdate,
        ProtocolVersion,
        RvdMessage,
    },
};
use std::{
    collections::HashMap,
    fmt::Debug,
    time::{Duration, Instant},
};

enum ShareTime {
    WaitingAck(Instant),
    Acked,
}

struct SharedDisplay {
    share_time: ShareTime,
    access_mask: AccessMask,
}

#[derive(Copy, Clone, Debug)]
pub enum HostState {
    Handshake,
    Ready,
}

// While the spec allows for each individual display to be controllable or not we only support all are controllable or none are
pub struct RvdHostHandler {
    state: HostState,
    permissions: PermissionMask,
    shared_displays: HashMap<DisplayId, SharedDisplay>,
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
            permissions: PermissionMask::empty(),
            shared_displays: HashMap::new(),
        }
    }

    pub fn protocol_version() -> RvdMessage {
        RvdMessage::ProtocolVersion(ProtocolVersion {
            version: RVD_VERSION.to_string(),
        })
    }

    pub fn set_permissions(&mut self, permissions: PermissionMask) -> RvdMessage {
        self.permissions = permissions;
        RvdMessage::PermissionsUpdate(PermissionsUpdate {
            permission_mask: permissions,
        })
    }

    fn find_unused_display_id(&self) -> Option<DisplayId> {
        for i in 0 .. u8::MAX {
            if !self.shared_displays.contains_key(&i) {
                return Some(i);
            }
        }
        None
    }

    pub fn share_display(
        &mut self,
        name: String,
        access: AccessMask,
    ) -> Result<(DisplayId, RvdMessage), RvdHostError> {
        let display_id = self
            .find_unused_display_id()
            .ok_or(RvdHostError::RanOutOfDisplayIds)?;

        let msg = RvdMessage::DisplayShare(DisplayShare {
            display_id,
            name,
            access,
        });

        self.shared_displays.insert(display_id, SharedDisplay {
            share_time: ShareTime::WaitingAck(Instant::now()),
            access_mask: access,
        });

        Ok((display_id, msg))
    }

    pub fn unshare_display(&mut self, display_id: DisplayId) -> Result<RvdMessage, RvdHostError> {
        if self.shared_displays.remove(&display_id).is_none() {
            return Err(RvdHostError::DisplayNotFound(display_id));
        }
        Ok(RvdMessage::DisplayUnshare(DisplayUnshare { display_id }))
    }

    /// This should be called every so often, at minimum probably every second.
    pub fn check_expired_shares(&mut self) -> Vec<RvdMessage> {
        let mut unshares = Vec::new();
        self.shared_displays
            .retain(|&display_id, share| match share.share_time {
                ShareTime::WaitingAck(start) =>
                    if start.elapsed() > Duration::from_secs(5) {
                        unshares.push(RvdMessage::DisplayUnshare(DisplayUnshare { display_id }));
                        false
                    } else {
                        true
                    },
                _ => true,
            });
        unshares
    }

    pub fn frame_update<'a>(&mut self, _pkt: &[u8]) -> impl Iterator<Item = RvdMessage> + 'a {
        /* let display_id = fragments.display_id;
        let shared_display = self
            .shared_displays
            .get_mut(&display_id)
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
        })*/
        Vec::new().into_iter()
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
                    self.state = HostState::Ready;
                    Ok(())
                }
                _ => Err(RvdHostError::WrongMessageForState(debug(&msg), self.state)),
            },
            HostState::Ready => match msg {
                RvdMessage::DisplayShareAck(msg) => {
                    let shared = match self.shared_displays.get_mut(&msg.display_id) {
                        None => return Ok(()),
                        Some(s) => s,
                    };
                    shared.share_time = ShareTime::Acked;
                    Ok(())
                }
                RvdMessage::MouseInput(msg) => {
                    let shared = match self.shared_displays.get(&msg.display_id) {
                        None => return Ok(()),
                        Some(s) => s,
                    };

                    if !shared.access_mask.contains(AccessMask::CONTROLLABLE) {
                        return Err(RvdHostError::PermissionsError(MouseInput));
                    }

                    events.push(InformEvent::RvdHostInform(RvdHostInform::MouseInput(
                        MouseInputEvent {
                            display_id: msg.display_id,
                            x_location: msg.x_location,
                            y_location: msg.y_location,
                            button_delta: msg.buttons_delta,
                            button_state: msg.buttons_state,
                        },
                    )));

                    Ok(())
                }
                RvdMessage::KeyInput(msg) => {
                    // TODO this is dumb, i'm losing brain cells here
                    if !self
                        .shared_displays
                        .iter()
                        .any(|(_, s)| s.access_mask.contains(AccessMask::CONTROLLABLE))
                    {
                        return Err(RvdHostError::PermissionsError(KeyInputPermission));
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
                    if !self.permissions.contains(PermissionMask::CLIPBOARD_READ) {
                        return Err(RvdHostError::PermissionsError(ClipboardRead));
                    }
                    events.push(InformEvent::RvdHostInform(RvdHostInform::ClipboardRequest(
                        msg.info.content_request,
                        msg.info.clipboard_type,
                    )));
                    Ok(())
                }
                RvdMessage::ClipboardNotification(msg) => {
                    if !self.permissions.contains(PermissionMask::CLIPBOARD_WRITE) {
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
    #[error("ran out of DisplayIDs. Are you sharing 256 displays?")]
    RanOutOfDisplayIds,
}

pub struct MouseInputEvent {
    pub display_id: DisplayId,
    pub x_location: u16,
    pub y_location: u16,
    pub button_delta: ButtonsMask,
    pub button_state: ButtonsMask,
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
