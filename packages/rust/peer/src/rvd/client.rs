use std::borrow::Cow;

use crate::{
    debug,
    helpers::crypto::{random_bytes, random_bytes_const},
    rvd::{HostState, RvdError, RvdHandlerTrait, RvdHostError},
    InformEvent,
    RvdHostInform,
};
use common::{
    constants::RVD_VERSION,
    messages::{
        rvd::{
            ClipboardType,
            DisplayId,
            DisplayShare,
            DisplayShareAck,
            FrameData,
            MouseLocation,
            ProtocolVersion,
            ProtocolVersionResponse,
            RvdMessage,
            UnreliableAuthFinal,
            UnreliableAuthInitial,
            UnreliableAuthInter,
        },
        Data,
    },
};

#[derive(Copy, Clone, Debug)]
pub enum ClientState {
    ProtocolVersion,
    UnreliableAuth([u8; 16], bool), // bool indicates whether handshake complete was received,
    // due to the edge case that the HandshakeComplete is received before UnreliableAuthFinal
    HandshakeComplete,
    Ready,
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
            state: ClientState::ProtocolVersion,
        }
    }

    pub fn protocol_version() -> RvdMessage<'static> {
        RvdMessage::ProtocolVersion(ProtocolVersion {
            version: RVD_VERSION.to_string(),
        })
    }

    pub fn _handle(
        &mut self,
        msg: RvdMessage<'_>,
        write: &mut Vec<RvdMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), RvdClientError> {
        match self.state {
            ClientState::ProtocolVersion => match msg {
                RvdMessage::ProtocolVersionResponse(msg) => {
                    if !msg.ok {
                        events.push(InformEvent::RvdClientInform(RvdClientInform::VersionBad));
                        return Ok(());
                    }
                    let challenge = random_bytes_const::<16>();
                    write.push(RvdMessage::UnreliableAuthInitial(UnreliableAuthInitial {
                        challenge: challenge.clone(),
                        zero: [0u8; 16],
                    }));
                    self.state = ClientState::UnreliableAuth(challenge, false);
                    Ok(())
                }
                _ => Err(RvdClientError::WrongMessageForState(
                    debug(&msg),
                    self.state,
                )),
            },
            ClientState::UnreliableAuth(challenge, complete) => match msg {
                RvdMessage::UnreliableAuthInter(msg) => {
                    if msg.response != challenge {
                        // TODO timing safe equal?
                        return Err(RvdClientError::UnreliableAuthFailed);
                    }
                    write.push(RvdMessage::UnreliableAuthFinal(UnreliableAuthFinal {
                        response: msg.challenge,
                    }));
                    if complete {
                        events.push(InformEvent::RvdClientInform(
                            RvdClientInform::HandshakeComplete,
                        ));
                        self.state = ClientState::Ready
                    } else {
                        self.state = ClientState::HandshakeComplete;
                    }
                    Ok(())
                }
                RvdMessage::HandshakeComplete(_) => {
                    self.state = ClientState::UnreliableAuth(challenge, true);
                    Ok(())
                }
                _ => Err(RvdClientError::WrongMessageForState(
                    debug(&msg),
                    self.state,
                )),
            },
            ClientState::HandshakeComplete => match msg {
                RvdMessage::HandshakeComplete { .. } => {
                    events.push(InformEvent::RvdClientInform(
                        RvdClientInform::HandshakeComplete,
                    ));
                    self.state = ClientState::Ready;
                    Ok(())
                }
                _ => Err(RvdClientError::WrongMessageForState(
                    debug(&msg),
                    self.state,
                )),
            },
            ClientState::Ready => match msg {
                RvdMessage::FrameData(msg) => {
                    events.push(InformEvent::RvdClientInform(RvdClientInform::FrameData(
                        FrameData {
                            display_id: msg.display_id,
                            data: Data(Cow::Owned(msg.data.0.into_owned())),
                        },
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
                RvdMessage::DisplayUnshare(msg) => {
                    events.push(InformEvent::RvdClientInform(
                        RvdClientInform::DisplayUnshare(msg.display_id),
                    ));
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
        msg: RvdMessage<'_>,
        write: &mut Vec<RvdMessage<'_>>,
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

    FrameData(FrameData<'static>),
    MouseHidden(DisplayId),
    MouseLocation(MouseLocation),
    DisplayShare(DisplayShare),
    DisplayUnshare(DisplayId),
    ClipboardNotification(Vec<u8>, ClipboardType), // for now we only care when receive a clipboard notification with content
}
