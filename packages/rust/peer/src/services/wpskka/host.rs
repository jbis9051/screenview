use crate::services::{
    helpers::{
        cipher_reliable_peer::{CipherError, CipherReliablePeer},
        cipher_unreliable_peer::CipherUnreliablePeer,
        wpskka_common::keypair,
    },
    wpskka::auth::{
        srp_host::{SrpAuthHost, SrpHostError},
        AuthScheme,
    },
    InformEvent,
    SendError,
};
use common::messages::{
    auth::srp::SrpMessage,
    wpskka::{AuthMessage, AuthSchemeType, TryAuth, WpskkaMessage},
    MessageComponent,
};
use ring::agreement::{EphemeralPrivateKey, PublicKey};
use std::{io::Cursor, sync::Arc};


#[derive(Copy, Clone, Debug)]
pub enum State {
    PreAuthSelect,
    IsAuthenticating,
    Authenticated,
}

pub struct WpskkaHostHandler {
    state: State,
    current_auth: Option<Box<AuthScheme>>,

    dynamic_password: Option<Vec<u8>>,
    static_password: Option<Vec<u8>>,

    reliable: Option<CipherReliablePeer>,
    unreliable: Option<Arc<CipherUnreliablePeer>>,

    keys: Option<Box<(EphemeralPrivateKey, PublicKey)>>,
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

    pub fn reliable_cipher_mut(&mut self) -> &mut CipherReliablePeer {
        self.reliable.as_mut().unwrap()
    }

    pub fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer> {
        self.unreliable.as_ref().unwrap()
    }

    pub fn handle_try_auth<F>(&mut self, msg: TryAuth, write: F) -> Result<(), WpskkaHostError>
    where F: Fn(WpskkaMessage) -> Result<(), SendError> {
        match msg.auth_scheme {
            AuthSchemeType::Invalid => Err(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme)),
            // These are basically the same schemes just with different password sources, so we can handle it together
            AuthSchemeType::SrpDynamic | AuthSchemeType::SrpStatic => {
                let password = if msg.auth_scheme == AuthSchemeType::SrpDynamic {
                    self.dynamic_password
                        .as_ref()
                        .ok_or(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme))?
                } else {
                    self.static_password
                        .as_ref()
                        .ok_or(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme))?
                };

                let keys = keypair().map_err(|_| WpskkaHostError::RingError)?;
                let mut srp = SrpAuthHost::new(keys.1.clone());

                let outgoing = srp.init(&password);
                write(WpskkaMessage::AuthMessage(AuthMessage {
                    data: outgoing
                        .to_bytes()
                        .map_err(|_| WpskkaHostError::BadAuthSchemeMessage)?,
                }))
                .map_err(WpskkaHostError::WriteError)?;

                self.current_auth = Some(Box::new(AuthScheme::SrpAuthHost(srp)));
                self.keys = Some(Box::new(keys));
                self.state = State::IsAuthenticating;

                Ok(())
            }
            AuthSchemeType::PublicKey => Err(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme)),
        }
    }

    pub fn handle<F>(
        &mut self,
        msg: WpskkaMessage,
        write: F,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError>
    where
        F: Fn(WpskkaMessage) -> Result<(), SendError>,
    {
        match self.state {
            State::PreAuthSelect => match msg {
                WpskkaMessage::TryAuth(msg) => {
                    self.handle_try_auth(msg, write)?;
                    Ok(None)
                }
                _ => Err(WpskkaHostError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
            State::IsAuthenticating => match msg {
                WpskkaMessage::TryAuth(msg) => {
                    self.current_auth = None; // TODO Security: Look into zeroing out the data here
                    self.handle_try_auth(msg, write)?;
                    Ok(None)
                }
                WpskkaMessage::AuthMessage(msg) => {
                    match &mut **self.current_auth.as_mut().unwrap() {
                        AuthScheme::SrpAuthHost(srp_host) => {
                            let msg = SrpMessage::read(&mut Cursor::new(&msg.data))
                                .map_err(|_| WpskkaHostError::BadAuthSchemeMessage)?;
                            match srp_host.handle(msg) {
                                Ok(outgoing) => {
                                    let data = outgoing
                                        .to_bytes()
                                        .map_err(|_| WpskkaHostError::BadAuthSchemeMessage)?;
                                    if srp_host.is_authenticated() {
                                        self.current_auth = None; // TODO Security: Look into zeroing out the data here
                                        self.state = State::Authenticated;
                                    }
                                    write(WpskkaMessage::AuthMessage(AuthMessage { data }))?;
                                    Ok(None)
                                }
                                Err(err) => match err {
                                    SrpHostError::WrongMessageForState(..) =>
                                        Err(WpskkaHostError::BadAuthSchemeMessage),
                                    _ => {
                                        events.push(InformEvent::WpskkaHostInform(
                                            WpskkaHostInform::AuthFailed,
                                        ));
                                        Ok(None)
                                    }
                                },
                            }
                        }
                        _ => {
                            panic!(
                                "Somehow an unsupported auth scheme ended up in the auth scheme. \
                                 Someone should get fired."
                            )
                        }
                    }
                }
                _ => Err(WpskkaHostError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
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
                _ => Err(WpskkaHostError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaHostError {
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(Box<WpskkaMessage>, State),

    #[error("unsupported auth scheme type")]
    BadAuthSchemeType(AuthSchemeType),
    #[error("BadAuthSchemeMessage")]
    BadAuthSchemeMessage,

    #[error("write error")]
    WriteError(#[from] SendError),

    #[error("ring error")]
    RingError,
    #[error("{0}")]
    CipherError(#[from] CipherError),
}

pub enum WpskkaHostInform {
    AuthFailed,
}
