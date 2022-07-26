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
        KeyState,
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
        TransportDataMessageReliable,
        TransportDataMessageUnreliable,
        TryAuth,
        WpskkaMessage,
    },
    Message,
    MessageComponent,
};
use ring::agreement::EphemeralPrivateKey;
use std::{
    fmt::{Debug, Formatter},
    io::Cursor,
    mem,
};

pub enum State {
    Modifying,
    PreInit,
    KeyExchange {
        key_pair: KeyPair,
    },
    PreAuthSelect {
        key_state: KeyState,
    },
    IsAuthenticating {
        // the auth holds onto the public key but it shouldn't need the private key so we hold on to it
        auth_scheme: AuthScheme<32>,
        private_key: EphemeralPrivateKey,
    },
    Authenticated {
        reliable: CipherReliablePeer,
        unreliable: CipherUnreliablePeer,
    },
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Modifying => write!(f, "Modifying"),
            State::PreInit => write!(f, "PreInit"),
            State::KeyExchange { .. } => write!(f, "KeyExchange {{ .. }}"),
            State::PreAuthSelect { .. } => write!(f, "PreAuthSelect {{ .. }}"),
            State::IsAuthenticating { .. } => write!(f, "IsAuthenticating {{ .. }}"),
            State::Authenticated { .. } => write!(f, "Authenticated {{ .. }}"),
        }
    }
}

pub struct WpskkaHostHandler {
    state: State,

    dynamic_password: Option<Vec<u8>>,
    static_password: Option<Vec<u8>>,
    none_scheme: bool,
}

impl Default for WpskkaHostHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl WpskkaHostHandler {
    pub fn new() -> Self {
        Self {
            state: State::PreInit,
            dynamic_password: None,
            static_password: None,
            none_scheme: false,
        }
    }

    pub fn handle_internal(
        &mut self,
        msg: WpskkaMessage<'_>,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match msg {
            WpskkaMessage::KeyExchange(msg) => self.handle_key_exchange(write, msg),
            WpskkaMessage::TryAuth(msg) => self.handle_try_auth(write, events, msg),
            WpskkaMessage::AuthMessage(msg) => self.handle_auth_message(write, events, msg),
            WpskkaMessage::TransportDataMessageReliable(msg) =>
                self.handle_message_reliable(write, events, msg),
            WpskkaMessage::TransportDataMessageUnreliable(msg) =>
                self.handle_message_unreliable(write, events, msg),
            msg => Err(WpskkaHostError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            )),
        }
    }

    fn handle_key_exchange(
        &mut self,
        write: &mut Vec<WpskkaMessage<'_>>,
        msg: KeyExchange,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        let key_pair = match mem::replace(&mut self.state, State::Modifying) {
            State::KeyExchange { key_pair } => key_pair,
            _ =>
                return Err(WpskkaHostError::WrongMessageForState(
                    debug(&msg),
                    debug(&self.state),
                )),
        };

        let client_public_key = msg.public_key;

        write.push(self.auth_schemes());

        self.state = State::PreAuthSelect {
            key_state: KeyState {
                foreign_public_key: client_public_key,
                key_pair,
            },
        };

        Ok(None)
    }

    /// DO NOT CALL THIS FUNCTION WITHOUT AUTHENTICATING THE FOREIGN PUBLIC KEY OR THE WORLD WILL END
    fn derive_keys(
        key_pair: KeyPair,
        foreign_public_key: [u8; 32],
    ) -> Result<(CipherReliablePeer, CipherUnreliablePeer), ()> {
        let client_public_key = parse_foreign_public(&foreign_public_key);
        let (send_reliable, receive_reliable, send_unreliable, receive_unreliable) =
            diffie_hellman(key_pair.ephemeral_private_key, client_public_key).map_err(|_| ())?;
        // TODO zero hella
        Ok((
            CipherReliablePeer::new(send_reliable.to_vec(), receive_reliable.to_vec()),
            CipherUnreliablePeer::new(send_unreliable.to_vec(), receive_unreliable.to_vec()),
        ))
    }

    fn derive_key_wrapper(key_state: KeyState) -> Result<State, (State, WpskkaHostError)> {
        match Self::derive_keys(key_state.key_pair, key_state.foreign_public_key) {
            Ok((reliable, unreliable)) => Ok(State::Authenticated {
                reliable,
                unreliable,
            }),
            Err(()) => Err((State::PreInit, WpskkaHostError::RingError)),
        }
    }

    fn handle_try_auth_stateless(
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
        dynamic_password: Option<&[u8]>,
        static_password: Option<&[u8]>,
        key_state: KeyState,
        auth_scheme: AuthSchemeType,
        none_scheme: bool,
    ) -> Result<State, (State, WpskkaHostError)> {
        match auth_scheme {
            AuthSchemeType::None =>
                if none_scheme {
                    let res = Self::derive_key_wrapper(key_state);
                    if res.is_ok() {
                        write.push(WpskkaMessage::AuthResult(AuthResult { ok: true }));
                        events.push(InformEvent::WpskkaHostInform(
                            WpskkaHostInform::AuthSuccessful,
                        ));
                    } else {
                        write.push(WpskkaMessage::AuthResult(AuthResult { ok: false }));
                        events.push(InformEvent::WpskkaHostInform(WpskkaHostInform::AuthFailed));
                    }
                    res
                } else {
                    write.push(WpskkaMessage::AuthResult(AuthResult { ok: false }));
                    events.push(InformEvent::WpskkaHostInform(WpskkaHostInform::AuthFailed));
                    Ok(State::PreAuthSelect { key_state })
                },
            // These are basically the same schemes just with different password sources, so we can handle it together
            AuthSchemeType::SrpDynamic | AuthSchemeType::SrpStatic => {
                let password = if auth_scheme == AuthSchemeType::SrpDynamic {
                    dynamic_password
                } else {
                    static_password
                };

                let password = match password {
                    None => {
                        write.push(WpskkaMessage::AuthResult(AuthResult { ok: false }));
                        return Ok(State::PreAuthSelect { key_state });
                    }
                    Some(password) => password,
                };

                let mut srp =
                    SrpAuthHost::new(key_state.key_pair.public_key, key_state.foreign_public_key);

                let message = srp.init(password);

                write.push(WpskkaMessage::AuthMessage(AuthMessage {
                    data: message
                        .to_bytes()
                        .expect("unable to convert srp message to bytes"),
                }));

                Ok(State::IsAuthenticating {
                    auth_scheme: AuthScheme::SrpAuthHost(srp),
                    private_key: key_state.key_pair.ephemeral_private_key,
                })
            }
            AuthSchemeType::PublicKey => {
                write.push(WpskkaMessage::AuthResult(AuthResult { ok: false }));
                Ok(State::PreAuthSelect { key_state })
            }
        }
    }

    fn handle_try_auth(
        &mut self,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
        msg: TryAuth,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        let key_state = match mem::replace(&mut self.state, State::Modifying) {
            State::PreAuthSelect { key_state } => key_state,
            // if we are authenticated we should cancel the auth and try the new one
            State::IsAuthenticating {
                auth_scheme,
                private_key: my_private_key,
            } => match auth_scheme {
                AuthScheme::SrpAuthHost(srp) => {
                    let (my_public, their_public) = srp.finish();
                    KeyState {
                        key_pair: KeyPair {
                            public_key: my_public,
                            ephemeral_private_key: my_private_key,
                        },
                        foreign_public_key: their_public,
                    }
                }
                _ => {
                    panic!(
                        "Somehow an unsupported auth scheme ended up in the auth scheme. Someone \
                         should get fired."
                    );
                }
            },
            _ =>
                return Err(WpskkaHostError::WrongMessageForState(
                    debug(&msg),
                    debug(&self.state),
                )),
        };

        match Self::handle_try_auth_stateless(
            write,
            events,
            self.dynamic_password.as_deref(),
            self.static_password.as_deref(),
            key_state,
            msg.auth_scheme,
            self.none_scheme,
        ) {
            Ok(state) => {
                self.state = state;
                Ok(None)
            }
            Err((state, err)) => {
                self.state = state;
                Err(err)
            }
        }
    }

    fn handle_auth_message(
        &mut self,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
        msg: AuthMessage,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match mem::replace(&mut self.state, State::Modifying) {
            State::IsAuthenticating {
                auth_scheme,
                private_key: my_private_key,
            } => match auth_scheme {
                AuthScheme::SrpAuthHost(mut srp_host) => {
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
                                let (my_pub, their_pub) = srp_host.finish();
                                let keypair = KeyPair {
                                    public_key: my_pub,
                                    ephemeral_private_key: my_private_key,
                                };
                                match Self::derive_key_wrapper(KeyState {
                                    key_pair: keypair,
                                    foreign_public_key: their_pub,
                                }) {
                                    Ok(state) => {
                                        write
                                            .push(WpskkaMessage::AuthMessage(AuthMessage { data }));
                                        write.push(WpskkaMessage::AuthResult(AuthResult {
                                            ok: true,
                                        }));
                                        self.state = state;
                                        Ok(None)
                                    }
                                    Err((state, err)) => {
                                        write.push(WpskkaMessage::AuthResult(AuthResult {
                                            ok: false,
                                        }));
                                        self.state = state;
                                        Err(err)
                                    }
                                }
                            } else {
                                write.push(WpskkaMessage::AuthMessage(AuthMessage { data }));
                                self.state = State::IsAuthenticating {
                                    auth_scheme: AuthScheme::SrpAuthHost(srp_host),
                                    private_key: my_private_key,
                                };
                                Ok(None)
                            }
                        }
                        Err(err) => match err {
                            SrpHostError::WrongMessageForState(..) =>
                                Err(WpskkaHostError::BadAuthSchemeMessage),
                            _ => {
                                write.push(WpskkaMessage::AuthResult(AuthResult { ok: false }));
                                events.push(InformEvent::WpskkaHostInform(
                                    WpskkaHostInform::AuthFailed,
                                ));
                                let (public_key, foreign_public_key) = srp_host.finish();

                                let key_pair = KeyPair {
                                    public_key,
                                    ephemeral_private_key: my_private_key,
                                };

                                self.state = State::PreAuthSelect {
                                    key_state: KeyState {
                                        key_pair,
                                        foreign_public_key,
                                    },
                                };

                                Ok(None)
                            }
                        },
                    }
                }
                _ => {
                    panic!(
                        "Somehow an unsupported auth scheme ended up in the auth scheme. Someone \
                         should get fired."
                    );
                }
            },
            State::Authenticated { .. } => Ok(None),
            _ => Err(WpskkaHostError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            )),
        }
    }

    fn handle_message_reliable(
        &mut self,
        _write: &mut [WpskkaMessage<'_>],
        _events: &mut [InformEvent],
        msg: TransportDataMessageReliable<'_>,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match self.state {
            State::Authenticated {
                ref mut reliable, ..
            } => Ok(Some(
                reliable
                    .decrypt(msg.data.0.as_ref())
                    .map_err(WpskkaHostError::CipherError)?,
            )),
            _ => Err(WpskkaHostError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            )),
        }
    }

    fn handle_message_unreliable(
        &mut self,
        _write: &mut [WpskkaMessage<'_>],
        _events: &mut [InformEvent],
        msg: TransportDataMessageUnreliable<'_>,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match &mut self.state {
            State::Authenticated { unreliable, .. } => Ok(Some(
                unreliable
                    .decrypt(msg.data.0.as_ref(), msg.counter)
                    .map_err(WpskkaHostError::CipherError)?,
            )),
            _ => Err(WpskkaHostError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            )),
        }
    }

    pub fn auth_schemes(&self) -> WpskkaMessage<'static> {
        let mut auth_schemes = Vec::new();
        if self.none_scheme {
            auth_schemes.push(AuthSchemeType::None);
        }
        if self.static_password.is_some() {
            auth_schemes.push(AuthSchemeType::SrpStatic);
        }
        if self.dynamic_password.is_some() {
            auth_schemes.push(AuthSchemeType::SrpDynamic);
        }
        WpskkaMessage::AuthScheme(AuthSchemeMessage { auth_schemes })
    }

    /// Creates a key_exchange message
    pub fn key_exchange(&mut self) -> Result<WpskkaMessage<'static>, WpskkaHostError> {
        if !matches!(self.state, State::PreInit) {
            return Err(WpskkaHostError::WrongState(
                debug(&self.state),
                debug(&State::PreInit),
            ));
        }
        let keys = keypair().map_err(|_| WpskkaHostError::RingError)?;
        let msg = WpskkaMessage::KeyExchange(KeyExchange {
            public_key: keys
                .public_key
                .as_ref()
                .try_into()
                .expect("public key could not be converted to array of 32 bytes"),
        });
        self.state = State::KeyExchange { key_pair: keys };
        Ok(msg)
    }

    pub fn set_dynamic_password(&mut self, dynamic_password: Option<Vec<u8>>) {
        self.dynamic_password = dynamic_password;
    }

    pub fn set_static_password(&mut self, static_password: Option<Vec<u8>>) {
        self.static_password = static_password;
    }

    /// Warning: Setting this to true allows ANY client to connect to the Host and should only be used in debug mode
    pub fn set_none_scheme(&mut self, allow_none: bool) {
        self.none_scheme = allow_none;
    }

    pub fn authenticated(&self) -> bool {
        matches!(self.state, State::Authenticated { .. })
    }
}

impl WpskkaHandlerTrait for WpskkaHostHandler {
    fn handle(
        &mut self,
        msg: WpskkaMessage<'_>,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaError> {
        let ret = self.handle_internal(msg, write, events).map_err(Into::into);
        assert!(!matches!(self.state, State::Modifying));
        ret
    }

    fn unreliable_cipher(&mut self) -> &mut CipherUnreliablePeer {
        match &mut self.state {
            State::Authenticated { unreliable, .. } => unreliable,
            _ => {
                panic!("Trying to get the unreliable cipher when not authenticated");
            }
        }
    }

    fn reliable_cipher_mut(&mut self) -> &mut CipherReliablePeer {
        match &mut self.state {
            State::Authenticated { reliable, .. } => reliable,
            _ => {
                panic!("Trying to get the reliable cipher when not authenticated");
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaHostError {
    #[error("invalid message {0} for state {1}")]
    WrongMessageForState(String, String),
    #[error("invalid state for function call: got {0} but expected {1}")]
    WrongState(String, String),

    #[error("unsupported auth scheme type")]
    BadAuthSchemeType(AuthSchemeType),
    #[error("BadAuthSchemeMessage")]
    BadAuthSchemeMessage,

    #[error("ring error")]
    RingError,
    #[error("{0}")]
    CipherError(#[from] CipherError),
}

#[derive(Debug)]
pub enum WpskkaHostInform {
    AuthSuccessful,
    AuthFailed,
}
