use crate::{
    debug,
    rvd::{RvdError, RvdHandlerTrait},
    InformEvent,
};
use common::{
    constants::RVD_VERSION,
    messages::rvd::{
        ClipboardType,
        DisplayChange,
        DisplayChangeReceived,
        DisplayId,
        FrameData,
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

pub struct RvdClientHandler {
    state: ClientState,
}

impl Default for RvdClientHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl RvdClientHandler {
    pub fn new() -> Self {
        Self {
            state: ClientState::Handshake,
        }
    }
}

impl RvdClientHandler {
    pub fn _handle(
        &mut self,
        msg: RvdMessage,
        write: &mut Vec<RvdMessage>,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdClientError> {
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
                    debug(&msg),
                    self.state,
                )),
            },
            ClientState::Data => match msg {
                RvdMessage::FrameData(msg) => {
                    events.push(InformEvent::RvdClientInform(RvdClientInform::FrameData(
                        msg,
                    )));
                    Ok(())
                }
                RvdMessage::DisplayChange(msg) => {
                    events.push(InformEvent::RvdClientInform(
                        RvdClientInform::DisplayChange(msg),
                    ));
                    write.push(RvdMessage::DisplayChangeReceived(DisplayChangeReceived {}));
                    Ok(())
                }
                RvdMessage::MouseHidden(msg) => {
                    events.push(InformEvent::RvdClientInform(RvdClientInform::MouseHidden(
                        msg.display_id,
                    )));
                    Ok(())
                }
                RvdMessage::MouseLocation(msg) => {
                    events.push(InformEvent::RvdClientInform(
                        RvdClientInform::MouseLocation(msg),
                    ));
                    Ok(())
                }
                RvdMessage::ClipboardNotification(msg) => {
                    if let Some(content) = msg.content {
                        events.push(InformEvent::RvdClientInform(
                            RvdClientInform::ClipboardNotification(
                                content,
                                msg.info.clipboard_type,
                            ),
                        ));
                    }
                    Ok(())
                }
                _ => Err(RvdClientError::WrongMessageForState(
                    debug(&msg),
                    self.state,
                )),
            },
        }
    }
}

impl RvdHandlerTrait for RvdClientHandler {
    fn handle(
        &mut self,
        msg: RvdMessage,
        write: &mut Vec<RvdMessage>,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdError> {
        Ok(self._handle(msg, write, events)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RvdClientError {
    #[error("invalid message {0} for state {1:?}")]
    WrongMessageForState(String, ClientState),
    #[error("permission error: cannot {0}")]
    PermissionsError(String),
}

pub enum RvdClientInform {
    VersionBad,

    FrameData(FrameData),
    MouseHidden(DisplayId),
    MouseLocation(MouseLocation),
    DisplayChange(DisplayChange),
    ClipboardNotification(Vec<u8>, ClipboardType), // for now we only care when receive a clipboard notification with content
}
