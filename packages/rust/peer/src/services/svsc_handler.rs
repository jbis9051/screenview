use crate::services::InformEvent;
use common::messages::svsc::{
    EstablishSessionStatus,
    ExpirationTime,
    LeaseExtensionResponse,
    LeaseResponseData,
    PeerId,
    SessionData,
    SessionDataSend,
    SvscMessage,
};

#[derive(Copy, Clone, Debug)]
pub enum State {
    Handshake,
    PostHandshake,
}

pub struct SvscHandler {
    state: State,
    lease: Option<LeaseResponseData>,
    session: Option<SessionData>,
    awaiting_lease_response: bool,
    awaiting_extension_response: bool,
    awaiting_session_response: bool,
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

    pub fn wrap(msg: Vec<u8>) -> SessionDataSend {
        SessionDataSend { data: msg }
    }

    pub fn peer_id(&self) -> Option<&PeerId> {
        Some(&self.session()?.peer_id)
    }

    pub fn lease(&self) -> Option<&LeaseResponseData> {
        self.lease.as_ref()
    }

    pub fn session(&self) -> Option<&SessionData> {
        self.session.as_ref()
    }

    pub fn handle(
        &mut self,
        msg: SvscMessage,
        event: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, SvscError> {
        match self.state {
            State::Handshake => match msg {
                // initial message
                SvscMessage::ProtocolVersionResponse(msg) => {
                    if !msg.ok {
                        return Err(SvscError::VersionBad);
                    }
                    self.state = State::PostHandshake;
                    Ok(None)
                }
                _ => Err(SvscError::WrongMessageForState(Box::new(msg), self.state)),
            },
            // SVSC is mostly stateless (or many messages can happen in any state), so lets just check by message instead
            _ => match msg {
                SvscMessage::LeaseResponse(msg) => {
                    if !self.awaiting_lease_response {
                        return Err(SvscError::WrongMessageForState(
                            Box::new(SvscMessage::LeaseResponse(msg)),
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
                            Box::new(SvscMessage::EstablishSessionResponse(msg)),
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
                    if self.awaiting_extension_response {
                        return Err(SvscError::UnexpectedExtension);
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

                _ => Err(SvscError::WrongMessageForState(Box::new(msg), self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SvscError {
    #[error("server rejected version")]
    VersionBad,
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(Box<SvscMessage>, State),
    #[error("unexpected extension response")]
    UnexpectedExtension,
}

pub enum SvscInform {
    LeaseUpdate,
    SessionUpdate,
    SessionEnd,

    LeaseRequestRejected,
    SessionRequestRejected(EstablishSessionStatus),
    LeaseExtensionRequestRejected,
}
