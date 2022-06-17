use crate::{
    debug,
    helpers::{
        cipher_reliable_peer::{CipherError, CipherReliablePeer},
        cipher_unreliable_peer::CipherUnreliablePeer,
        crypto::{diffie_hellman, keypair, parse_foreign_public, KeyPair},
    },
    wpskka::{
        auth::{
            srp_host::{SrpAuthHost, SrpHostError},
            AuthScheme,
        },
        WpskkaError,
        WpskkaHandlerTrait,
    },
    InformEvent,
};
use common::messages::{
    auth::srp::SrpMessage,
    wpskka::{
        AuthMessage,
        AuthResult,
        AuthScheme as AuthSchemeMessage,
        AuthSchemeType,
        KeyExchange,
        TryAuth,
        WpskkaMessage,
    },
    Message,
    MessageComponent,
};
use std::{
    fmt::{Debug, Formatter},
    io::Cursor,
    rc::Rc,
    sync::Arc,
};

struct KeyState {
    client_public_key: [u8; 32],
    my_key_pair: KeyPair,
}

impl Debug for KeyState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeyState {{ <private> }}")
    }
}

#[derive(Debug)]
pub enum State {
    KeyExchange,
    PreAuthSelect {
        key_state: Rc<KeyState>,
    },
    IsAuthenticating {
        key_state: Rc<KeyState>,
        auth_scheme: Box<AuthScheme>,
    },
    Authenticated {
        key_state: Rc<KeyState>,
    },
}

pub struct WpskkaHostHandler {
    state: State,

    dynamic_password: Option<Vec<u8>>,
    static_password: Option<Vec<u8>>,
    none_scheme: bool,

    reliable: Option<CipherReliablePeer>,
    unreliable: Option<Arc<CipherUnreliablePeer>>,

    keys: Option<Box<KeyPair>>,
}

impl Default for WpskkaHostHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl WpskkaHostHandler {
    pub fn new() -> Self {
        Self {
            state: State::KeyExchange,
            dynamic_password: None,
            static_password: None,
            none_scheme: false,
            reliable: None,
            unreliable: None,
            keys: None,
        }
    }

    /// Warning: this erases and regenerates keys on every call
    pub fn auth_schemes(&mut self) -> Result<WpskkaMessage<'static>, WpskkaHostError> {
        let mut auth_schemes = Vec::new();
        if self.static_password.is_some() {
            auth_schemes.push(AuthSchemeType::SrpStatic);
        }
        if self.dynamic_password.is_some() {
            auth_schemes.push(AuthSchemeType::SrpDynamic);
        }
        let msg = WpskkaMessage::AuthScheme(AuthSchemeMessage { auth_schemes });
        Ok(msg)
    }

    pub fn handle_try_auth(
        &mut self,
        msg: TryAuth,
        write: &mut Vec<WpskkaMessage<'_>>,
        key_state: &Rc<KeyState>,
    ) -> Result<(), WpskkaHostError> {
        match msg.auth_scheme {
            AuthSchemeType::None => {
                write.push(WpskkaMessage::AuthResult(AuthResult {
                    ok: self.none_scheme,
                }));
                if self.none_scheme {
                    self.state = State::Authenticated {
                        key_state: key_state.clone(),
                    };
                }
                Err(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme))
            }
            // These are basically the same schemes just with different password sources, so we can handle it together
            AuthSchemeType::SrpDynamic | AuthSchemeType::SrpStatic => {
                /*  let password = if msg.auth_scheme == AuthSchemeType::SrpDynamic {
                    self.dynamic_password
                        .as_ref()
                        .ok_or(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme))?
                } else {
                    self.static_password
                        .as_ref()
                        .ok_or(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme))?
                };

                let mut srp = SrpAuthHost::new(
                    self.keys.as_ref().unwrap().public_key.clone(),
                    msg.public_key.to_vec(),
                );

                let outgoing = srp.init(password);
                write.push(WpskkaMessage::AuthMessage(AuthMessage {
                    data: outgoing
                        .to_bytes()
                        .map_err(|_| WpskkaHostError::BadAuthSchemeMessage)?,
                }));

                self.current_auth = Some(Box::new(AuthScheme::SrpAuthHost(srp)));
                self.state = State::IsAuthenticating;*/

                Ok(())
            }
            AuthSchemeType::PublicKey => Err(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme)),
        }
    }

    pub fn _handle(
        &mut self,
        msg: WpskkaMessage<'_>,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match self.state {
            State::KeyExchange => match msg {
                WpskkaMessage::KeyExchange(KeyExchange { public_key }) => {
                    let keys = keypair().map_err(|_| WpskkaHostError::RingError)?;

                    write.push(WpskkaMessage::KeyExchange(KeyExchange {
                        public_key: keys
                            .public_key
                            .as_ref()
                            .try_into()
                            .expect("unable to convert public key to 32 byte array"),
                    }));

                    self.state = State::PreAuthSelect {
                        key_state: Rc::new(KeyState {
                            client_public_key: public_key,
                            my_key_pair: keys,
                        }),
                    };

                    Ok(None)
                }
                _ => Err(WpskkaHostError::WrongMessageForState(
                    debug(&msg),
                    debug(&self.state),
                )),
            },
            State::PreAuthSelect { ref key_state } => match msg {
                WpskkaMessage::TryAuth(msg) => {
                    self.handle_try_auth(msg, write, key_state)?;
                    Ok(None)
                }
                _ => Err(WpskkaHostError::WrongMessageForState(
                    debug(&msg),
                    debug(&self.state),
                )),
            },
            State::IsAuthenticating {
                mut key_state,
                mut auth_scheme,
            } => match msg {
                WpskkaMessage::TryAuth(msg) => {
                    // TODO look into clearing auth_scheme data
                    self.state = State::PreAuthSelect {
                        key_state: key_state.clone(),
                    };
                    self.handle_try_auth(msg, write, &mut key_state)?;
                    Ok(None)
                }
                WpskkaMessage::AuthMessage(msg) => match auth_scheme.as_mut() {
                    AuthScheme::SrpAuthHost(srp_host) => {
                        let msg = SrpMessage::read(&mut Cursor::new(&msg.data))
                            .map_err(|_| WpskkaHostError::BadAuthSchemeMessage)?;
                        match srp_host.handle(msg) {
                            Ok(outgoing) => {
                                let data = outgoing
                                    .to_bytes()
                                    .map_err(|_| WpskkaHostError::BadAuthSchemeMessage)?;
                                if srp_host.is_authenticated() {
                                    events.push(InformEvent::WpskkaHostInform(
                                        WpskkaHostInform::AuthSuccessful,
                                    ));
                                    Self::derive_keys(
                                        key_state.my_key_pair,
                                        key_state.client_public_key,
                                    )?;
                                }
                                write.push(WpskkaMessage::AuthMessage(AuthMessage { data }));
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
                },
                _ => Err(WpskkaHostError::WrongMessageForState(
                    debug(&msg),
                    debug(&self.state),
                )),
            },
            State::Authenticated { key_state } => match msg {
                WpskkaMessage::TransportDataMessageReliable(msg) => {
                    let reliable = self.reliable.as_mut().unwrap();
                    Ok(Some(
                        reliable
                            .decrypt(msg.data.0.as_ref())
                            .map_err(WpskkaHostError::CipherError)?,
                    ))
                }
                WpskkaMessage::TransportDataMessageUnreliable(msg) => {
                    let unreliable = self.unreliable.as_mut().unwrap();
                    Ok(Some(
                        unreliable
                            .decrypt(msg.data.0.as_ref(), msg.counter)
                            .map_err(WpskkaHostError::CipherError)?,
                    ))
                }
                _ => Err(WpskkaHostError::WrongMessageForState(
                    debug(&msg),
                    debug(&self.state),
                )),
            },
        }
    }

    pub fn set_dynamic_password(&mut self, dynamic_password: Option<Vec<u8>>) {
        self.dynamic_password = dynamic_password;
    }

    pub fn set_static_password(&mut self, static_password: Option<Vec<u8>>) {
        self.static_password = static_password;
    }

    /// Warning: This is allows ANY client to connect to the Host and should only be used in debug mode
    pub fn set_none_scheme(&mut self, allow_none: bool) {
        self.none_scheme = allow_none;
    }

    pub fn authenticated(&self) -> bool {
        matches!(self.state, State::Authenticated { .. })
    }

    /// DO NOT CALL THIS FUNCTION WITHOUT AUTHENTICATING THE FOREIGN PUBLIC KEY OR THE WORLD WILL END
    fn derive_keys(
        key_pair: KeyPair,
        client_public_key: [u8; 32],
    ) -> Result<(CipherReliablePeer, CipherUnreliablePeer), WpskkaHostError> {
        let client_public_key = parse_foreign_public(&client_public_key);
        let (send_reliable, receive_reliable, send_unreliable, receive_unreliable) =
            diffie_hellman(key_pair.ephemeral_private_key, client_public_key)
                .map_err(|_| WpskkaHostError::RingError)?;
        // TODO zero hella
        Ok((
            CipherReliablePeer::new(send_reliable.to_vec(), receive_reliable.to_vec()),
            CipherUnreliablePeer::new(send_unreliable.to_vec(), receive_unreliable.to_vec()),
        ))
    }
}

impl WpskkaHandlerTrait for WpskkaHostHandler {
    fn handle(
        &mut self,
        msg: WpskkaMessage<'_>,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaError> {
        Ok(self._handle(msg, write, events)?)
    }

    fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer> {
        self.unreliable.as_ref().unwrap()
    }

    fn reliable_cipher_mut(&mut self) -> &mut CipherReliablePeer {
        self.reliable.as_mut().unwrap()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaHostError {
    #[error("invalid message {0} for state {0}")]
    WrongMessageForState(String, String),

    #[error("unsupported auth scheme type")]
    BadAuthSchemeType(AuthSchemeType),
    #[error("BadAuthSchemeMessage")]
    BadAuthSchemeMessage,

    #[error("ring error")]
    RingError,
    #[error("{0}")]
    CipherError(#[from] CipherError),
}

pub enum WpskkaHostInform {
    AuthSuccessful,
    AuthFailed,
}
