use crate::{
    debug,
    helpers::crypto::{random_bytes, random_bytes_const},
    rvd::{RvdError, RvdHandlerTrait},
    InformEvent,
};
use common::{
    constants::RVD_VERSION,
    messages::rvd::{
        ClipboardType,
        DisplayId,
        DisplayShare,
        DisplayShareAck,
        FrameData,
        MouseLocation,
        ProtocolVersionResponse,
        RvdMessage,
        UnreliableAuthInter,
    },
};

#[derive(Copy, Clone, Debug)]
pub enum ClientState {
    AwaitingProtocolVersion,
    InUnreliableAuthStep1,
    InUnreliableAuthStep2([u8; 16]),
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
            state: ClientState::AwaitingProtocolVersion,
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
            ClientState::AwaitingProtocolVersion => match msg {
                RvdMessage::ProtocolVersion(msg) => {
                    let ok = msg.version == RVD_VERSION;
                    write.push(RvdMessage::ProtocolVersionResponse(
                        ProtocolVersionResponse { ok },
                    ));
                    if ok {
                        self.state = ClientState::InUnreliableAuthStep1;
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
            ClientState::InUnreliableAuthStep1 => match msg {
                RvdMessage::UnreliableAuthInitial(msg) => {
                    let challenge = random_bytes_const::<16>();
                    write.push(RvdMessage::UnreliableAuthInter(UnreliableAuthInter {
                        response: msg.challenge,
                        challenge: challenge.clone(),
                    }));
                    self.state = ClientState::InUnreliableAuthStep2(challenge);
                    Ok(())
                }
                _ => Err(RvdClientError::WrongMessageForState(
                    debug(&msg),
                    self.state,
                )),
            },
            ClientState::InUnreliableAuthStep2(challenge) => match msg {
                RvdMessage::UnreliableAuthFinal(msg) => {
                    let ok = msg.response == challenge;
                    if ok {
                        self.state = ClientState::Data;
                        events.push(InformEvent::RvdClientInform(
                            RvdClientInform::HandshakeComplete,
                        ));
                    } else {
                        return Err(RvdClientError::UnreliableAuthFailed);
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
                RvdMessage::DisplayShare(msg) => {
                    write.push(RvdMessage::DisplayShareAck(DisplayShareAck {
                        display_id: msg.display_id,
                    }));
                    events.push(InformEvent::RvdClientInform(RvdClientInform::DisplayShare(
                        msg,
                    )));
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
    #[error("unreliable auth failed")]
    UnreliableAuthFailed,
}

#[derive(Debug)]
pub enum RvdClientInform {
    VersionBad,

    HandshakeComplete,

    FrameData(FrameData),
    MouseHidden(DisplayId),
    MouseLocation(MouseLocation),
    DisplayShare(DisplayShare),
    ClipboardNotification(Vec<u8>, ClipboardType), // for now we only care when receive a clipboard notification with content
}
