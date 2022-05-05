use crate::{debug, InformEvent};
use common::{
    constants::SVSC_VERSION,
    messages::{
        svsc::{
            Cookie,
            EstablishSessionRequest,
            EstablishSessionStatus,
            KeepAlive,
            LeaseExtensionRequest,
            LeaseId,
            LeaseRequest,
            LeaseResponseData,
            PeerId,
            ProtocolVersionResponse,
            SessionData,
            SessionDataSend,
            SvscMessage,
        },
        Data,
    },
};
use std::{borrow::Cow, fmt::Debug};

#[derive(Copy, Clone, Debug)]
pub enum State {
    Handshake,
    PostHandshake,
}

pub struct SvscHandler {
    state: State,
    lease: Option<LeaseResponseData>,
    session: Option<Box<SessionData>>,
    awaiting_lease_response: bool,
    awaiting_extension_response: bool,
    awaiting_session_response: bool,
}

impl Default for SvscHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SvscHandler {
    pub fn new() -> Self {
        Self {
            state: State::Handshake,
            lease: None,
            session: None,
            awaiting_lease_response: false,
            awaiting_extension_response: false,
            awaiting_session_response: false,
        }
    }

    pub fn wrap(msg: Vec<u8>) -> SessionDataSend<'static> {
        SessionDataSend {
            data: Data(Cow::Owned(msg)),
        }
    }

    pub fn peer_id(&self) -> Option<&PeerId> {
        Some(&self.session()?.peer_id)
    }

    pub fn lease(&self) -> Option<&LeaseResponseData> {
        self.lease.as_ref()
    }

    pub fn session(&self) -> Option<&SessionData> {
        self.session.as_deref()
    }

    pub fn lease_request(&mut self, cookie: Option<Cookie>) -> SvscMessage {
        self.awaiting_lease_response = true;
        SvscMessage::LeaseRequest(LeaseRequest { cookie })
    }

    pub fn lease_extension_request(&mut self, cookie: Cookie) -> SvscMessage {
        self.awaiting_extension_response = true;
        SvscMessage::LeaseExtensionRequest(LeaseExtensionRequest { cookie })
    }

    pub fn establish_session_request(&mut self, lease_id: LeaseId) -> SvscMessage {
        self.awaiting_session_response = true;
        SvscMessage::EstablishSessionRequest(EstablishSessionRequest { lease_id })
    }

    pub fn handle<'a>(
        &mut self,
        msg: SvscMessage<'a>,
        write: &mut Vec<SvscMessage>,
        event: &mut Vec<InformEvent>,
    ) -> Result<Option<Cow<'a, [u8]>>, SvscError> {
        if let SvscMessage::KeepAlive(_) = msg {
            write.push(SvscMessage::KeepAlive(KeepAlive {}));
            return Ok(None);
        }
        match self.state {
            State::Handshake => match msg {
                // initial message
                SvscMessage::ProtocolVersion(msg) => {
                    let check = msg.version == SVSC_VERSION; // TODO check version
                    write.push(SvscMessage::ProtocolVersionResponse(
                        ProtocolVersionResponse { ok: check },
                    ));
                    if !check {
                        event.push(InformEvent::SvscInform(SvscInform::VersionBad));
                    } else {
                        self.state = State::PostHandshake;
                    }
                    Ok(None)
                }
                _ => Err(SvscError::WrongMessageForState(debug(&msg), self.state)),
            },
            // SVSC is mostly stateless (or many messages can happen in any state), so lets just check by message instead
            _ => match msg {
                SvscMessage::LeaseResponse(msg) => {
                    if !self.awaiting_lease_response {
                        return Err(SvscError::WrongMessageForState(
                            debug(&SvscMessage::LeaseResponse(msg)),
                            self.state,
                        ));
                    }
                    self.awaiting_lease_response = false;
                    event.push(InformEvent::SvscInform(match msg.response_data {
                        None => SvscInform::LeaseRequestRejected,
                        Some(data) => {
                            self.lease = Some(data);
                            SvscInform::LeaseUpdate
                        }
                    }));
                    Ok(None)
                }

                SvscMessage::EstablishSessionResponse(msg) => {
                    if !self.awaiting_session_response {
                        return Err(SvscError::WrongMessageForState(
                            debug(&SvscMessage::EstablishSessionResponse(msg)),
                            self.state,
                        ));
                    }
                    self.awaiting_session_response = false;
                    event.push(InformEvent::SvscInform(match msg.response_data {
                        None => SvscInform::SessionRequestRejected(msg.status),
                        Some(data) => {
                            self.session = Some(data);
                            SvscInform::SessionUpdate
                        }
                    }));
                    Ok(None)
                }

                SvscMessage::LeaseExtensionResponse(msg) => {
                    if !self.awaiting_extension_response {
                        return Err(SvscError::WrongMessageForState(
                            debug(&SvscMessage::LeaseExtensionResponse(msg)),
                            self.state,
                        ));
                    }
                    self.awaiting_extension_response = false;
                    event.push(InformEvent::SvscInform(match msg.new_expiration {
                        None => SvscInform::LeaseExtensionRequestRejected,
                        Some(expiration) => {
                            let mut lease = self.lease.as_mut().unwrap();
                            lease.expiration = expiration;
                            SvscInform::LeaseUpdate
                        }
                    }));
                    Ok(None)
                }

                SvscMessage::EstablishSessionNotification(msg) => {
                    event.push(InformEvent::SvscInform(SvscInform::SessionUpdate));
                    self.session = Some(Box::new(msg.session_data));
                    Ok(None)
                }

                SvscMessage::SessionEnd(_msg) => {
                    // TODO should we error if session doesn't exist
                    event.push(InformEvent::SvscInform(SvscInform::SessionUpdate));
                    self.session = None;
                    Ok(None)
                }

                SvscMessage::SessionDataReceive(msg) => {
                    if self.session.is_none() {
                        return Err(SvscError::WrongMessageForState(
                            debug(&SvscMessage::SessionDataReceive(msg)),
                            self.state,
                        ));
                    }
                    Ok(Some(msg.data.0))
                }

                _ => Err(SvscError::WrongMessageForState(debug(&msg), self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SvscError {
    #[error("invalid message {0} for state {1:?}")]
    WrongMessageForState(String, State),
}

pub enum SvscInform {
    VersionBad,

    LeaseUpdate,
    SessionUpdate,
    SessionEnd,

    LeaseRequestRejected,
    SessionRequestRejected(EstablishSessionStatus),
    LeaseExtensionRequestRejected,
}
