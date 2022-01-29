use crate::services::{
    helpers::{
        cipher_reliable_peer::{CipherError, CipherReliablePeer},
        cipher_unreliable_peer::CipherUnreliablePeer,
        wpskka_common::{hmac, hmac_verify, kdf1, keypair, random_bytes, random_srp_private_value},
    },
    wpskka::auth::{
        srp_host::{SrpAuthHost, SrpHostError},
        AuthScheme,
    },
};
use common::{
    constants::{HashAlgo, SRP_PARAM},
    messages::{
        auth::srp::SrpMessage,
        wpskka::{AuthSchemeType, HostHello, HostVerify, WpskkaMessage},
        ScreenViewMessage,
    },
};
use ring::agreement::{EphemeralPrivateKey, PublicKey};
use std::sync::mpsc::{SendError, Sender};

#[derive(Copy, Clone, Debug)]
pub enum State {
    PreAuthSelect,
    IsAuthenticating,
    Authenticated,
}

pub struct WpskkaHostHandler {
    state: State,
    current_auth: Option<AuthScheme>,

    dynamic_password: Option<Vec<u8>>,
    static_password: Option<Vec<u8>>,

    reliable: Option<CipherReliablePeer>,
    unreliable: Option<CipherUnreliablePeer>,

    keys: Option<(EphemeralPrivateKey, PublicKey)>,
}

impl WpskkaHostHandler {
    pub fn new() -> Self {
        Self {
            state: State::PreAuthSelect,
            current_auth: None,
            dynamic_password: None,
            static_password: None,
            reliable: None,
            unreliable: None,
            keys: None,
        }
    }

    pub fn handle(
        &mut self,
        msg: WpskkaMessage,
        write: &mut Sender<ScreenViewMessage>,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match self.state {
            State::PreAuthSelect => match msg {
                WpskkaMessage::TryAuth(msg) => {
                    match msg.auth_scheme {
                        AuthSchemeType::Invalid =>
                            Err(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme)),
                        // These are basically the same schemes just with different password sources, so we can handle it together
                        AuthSchemeType::SrpDynamic | AuthSchemeType::SrpStatic => {
                            let password = if msg.auth_scheme == AuthSchemeType::SrpDynamic {
                                self.dynamic_password
                                    .ok_or(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme))?
                            } else {
                                self.static_password
                                    .ok_or(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme))?
                            };

                            let keys = keypair().map_err(|_| WpskkaHostError::RingError)?;
                            let mut srp = SrpAuthHost::new(keys.1.clone());

                            let outgoing = srp.init(&password);
                            write
                                .send(ScreenViewMessage::WpskkaMessage(
                                    WpskkaMessage::AuthMessage(outgoing.to_bytes()),
                                ))
                                .map_err(WpskkaHostError::WriteError)?;
                            self.current_auth = Some(AuthScheme::SrpAuthHost(srp));
                            self.keys = Some(keys);
                            self.state = State::IsAuthenticating;
                            Ok(None)
                        }
                        AuthSchemeType::PublicKey =>
                            Err(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme)),
                    }
                }
                _ => Err(WpskkaHostError::WrongMessageForState(msg, self.state)),
            },
            State::IsAuthenticating => match msg {
                WpskkaMessage::AuthMessage(msg) => {
                    match self.current_auth.unwrap() {
                        AuthScheme::SrpAuthHost(mut host) => {
                            let msg: SrpMessage = SrpMessage::read(&msg.data);
                            let outgoing = host.handle(msg).map_err(|err| {
                                if err == SrpHostError::WrongMessageForState {
                                    WpskkaHostError::BadAuthSchemeMessage
                                } else {
                                    WpskkaHostError::AuthFailed
                                }
                            })?;
                            write
                                .send(ScreenViewMessage::WpskkaMessage(
                                    WpskkaMessage::AuthMessage(outgoing.to_bytes()),
                                ))
                                .map_err(WpskkaHostError::WriteError)?;
                            if host.is_authenticated() {
                                self.current_auth = None; // TODO Security: Look into zeroing out the data here
                                self.state = State::Authenticated;
                            }
                            Ok(None)
                        }
                        _ => {
                            panic!(
                                "Somehow an unsupported auth scheme ended up in the auth scheme. \
                                 Someone should get fired."
                            )
                        }
                    }
                }
                _ => Err(WpskkaHostError::WrongMessageForState(msg, self.state)),
            },
            State::Authenticated => match msg {
                WpskkaMessage::TransportDataMessageReliable(msg) => {
                    let reliable = self.reliable.as_mut().unwrap();
                    Ok(Some(
                        reliable
                            .decrypt(msg.data)
                            .map_err(WpskkaHostError::CipherError)?,
                    ))
                }
                WpskkaMessage::TransportDataMessageUnreliable(msg) => {
                    let unreliable = self.unreliable.as_mut().unwrap();
                    Ok(Some(
                        unreliable
                            .decrypt(msg.data, msg.counter)
                            .map_err(WpskkaHostError::CipherError)?,
                    ))
                }
                _ => Err(WpskkaHostError::WrongMessageForState(msg, self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaHostError {
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(WpskkaMessage, State),

    #[error("unsupported auth scheme type")]
    BadAuthSchemeType(AuthSchemeType),
    #[error("authentication failed")]
    AuthFailed,
    #[error("BadAuthSchemeMessage")]
    BadAuthSchemeMessage,

    #[error("inform error")]
    InformError(SendError<ScreenViewMessage>),
    #[error("write error")]
    WriteError(SendError<ScreenViewMessage>),

    #[error("ring error")]
    RingError,
    #[error("{0}")]
    CipherError(CipherError),
}
