use common::messages::svsc::{
    ExpirationTime, LeaseExtensionResponse, LeaseResponseData, SessionData, SvscMessage,
};

#[derive(Copy, Clone, Debug)]
enum State {
    Handshake,
    PreSession, // limbo land, we shouldn't receive any messages
    AwaitingLeaseResponse,
    AwaitingSessionResponse,
    InLease, // we have an ID assigned
    InSession,
}

pub struct SvscHandler {
    state: State,
    awaiting_extension_response: bool,
    lease: Option<LeaseResponseData>,
    session: Option<SessionData>,
}

impl SvscHandler {
    pub fn new() -> Self {
        Self {
            state: State::Handshake,
            awaiting_extension_response: false,
            lease: None,
            session: None,
        }
    }

    pub fn handle(&mut self, msg: SvscMessage) -> Result<(Option<Vec<u8>>, SvscInform), SvscError> {
        match self.state {
            State::Handshake => match msg {
                SvscMessage::ProtocolVersionResponse(msg) => {
                    if !msg.ok {
                        return Err(SvscError::VersionBad);
                    }
                    self.state = State::PreSession;
                    Ok((None, SvscInform::None))
                }
                _ => Err(SvscError::WrongMessageForState(msg, self.state)),
            },
            State::PreSession => Err(SvscError::WrongMessageForState(msg, self.state)),
            State::AwaitingLeaseResponse => match msg {
                SvscMessage::LeaseResponse(msg) => {
                    let data = msg.response_data.ok_or(SvscError::LeaseRequestRejected)?;
                    self.lease = Some(data);
                    self.state = State::InLease;
                    Ok((None, SvscInform::LeaseUpdate(data)))
                }
                _ => Err(SvscError::WrongMessageForState(msg, self.state)),
            },
            State::AwaitingSessionResponse => match msg {
                SvscMessage::EstablishSessionResponse(msg) => {
                    let data = msg.response_data.ok_or(SvscError::LeaseRequestRejected)?;
                    self.session = Some(data);
                    Ok((None, SvscInform::SessionUpdate(data)))
                }
                _ => Err(SvscError::WrongMessageForState(msg, self.state)),
            },
            State::InLease => match msg {
                SvscMessage::LeaseExtensionResponse(msg) => {
                    self.handle_lease_extension_response(msg)
                }
                SvscMessage::EstablishSessionNotification(msg) => {
                    self.session = Some(msg.session_data);
                    Ok((None, SvscInform::SessionUpdate(msg.session_data)))
                }
                _ => Err(SvscError::WrongMessageForState(msg, self.state)),
            },
            State::InSession => match msg {
                SvscMessage::LeaseExtensionResponse(msg) => {
                    self.handle_lease_extension_response(msg)
                }
                SvscMessage::SessionEndNotification(_) => {
                    if self.lease.is_some() {
                        self.state = State::InLease;
                    }
                    self.session = None;
                    Ok((None, SvscInform::SessionEnd))
                }
                SvscMessage::SessionDataReceive(msg) => Ok((Some(msg.data), SvscInform::None)),
                _ => Err(SvscError::WrongMessageForState(msg, self.state)),
            },
        }
    }

    fn handle_lease_extension_response(
        &mut self,
        msg: LeaseExtensionResponse,
    ) -> Result<(Option<Vec<u8>>, SvscInform), SvscError> {
        if self.awaiting_extension_response {
            return Err(SvscError::UnexpectedExtension);
        }
        let expiration = msg
            .new_expiration
            .ok_or(SvscError::LeaseExtensionRequestRejected)?;
        self.lease.unwrap().expiration = expiration;
        self.awaiting_extension_response = false;
        Ok((None, SvscInform::None))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SvscError {
    #[error("server rejected version")]
    VersionBad,
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(SvscMessage, State),
    #[error("unexpected extension response")]
    UnexpectedExtension,
    #[error("lease request was rejected")]
    LeaseRequestRejected,
    #[error("lease extension request was rejected")]
    LeaseExtensionRequestRejected,
}

pub enum SvscInform {
    None,
    LeaseUpdate(LeaseResponseData),
    SessionUpdate(SessionData),
    SessionEnd,
}
