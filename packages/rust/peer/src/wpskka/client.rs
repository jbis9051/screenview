use crate::{
    debug,
    helpers::{
        cipher_reliable_peer::{CipherError, CipherReliablePeer},
        cipher_unreliable_peer::CipherUnreliablePeer,
        crypto::{diffie_hellman, keypair, parse_foreign_public, KeyPair},
    },
    wpskka::{
        auth::{
            srp_client::{SrpAuthClient, SrpClientError},
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
    KeyExchange,
    ChooseAnAuthScheme {
        key_pair: KeyPair,
        foreign_public_key: [u8; 32],
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
            State::KeyExchange => write!(f, "KeyExchange"),
            State::ChooseAnAuthScheme { .. } => write!(f, "WaitingForAuthSchemes {{ .. }}"),
            State::IsAuthenticating { .. } => write!(f, "IsAuthenticating {{ .. }}"),
            State::Authenticated { .. } => write!(f, "Authenticated {{ .. }}"),
        }
    }
}

pub struct WpskkaClientHandler {
    state: State,
}

impl Default for WpskkaClientHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl WpskkaClientHandler {
    pub fn new() -> Self {
        Self {
            state: State::KeyExchange,
        }
    }

    pub fn handle_internal(
        &mut self,
        msg: WpskkaMessage<'_>,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        match msg {
            WpskkaMessage::KeyExchange(msg) => self.handle_key_exchange(msg, write),
            WpskkaMessage::AuthScheme(msg) => self.handle_auth_scheme(msg, events),
            WpskkaMessage::AuthMessage(msg) => self.handle_auth_message(msg, write, events),
            WpskkaMessage::AuthResult(msg) => self.handle_auth_result(msg, write, events),
            WpskkaMessage::TransportDataMessageReliable(msg) =>
                self.handle_message_reliable(msg, write, events),
            WpskkaMessage::TransportDataMessageUnreliable(msg) =>
                self.handle_message_unreliable(msg, write, events),
            _ => Err(WpskkaClientError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            )),
        }
    }

    fn handle_key_exchange(
        &mut self,
        msg: KeyExchange,
        write: &mut Vec<WpskkaMessage<'_>>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        if !matches!(self.state, State::KeyExchange) {
            return Err(WpskkaClientError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            ));
        }
        let host_public_key = msg.public_key;
        let keys = keypair().map_err(|_| WpskkaClientError::RingError)?;

        write.push(WpskkaMessage::KeyExchange(KeyExchange {
            public_key: keys
                .public_key
                .as_ref()
                .try_into()
                .expect("public key could not be converted to array of 32 bytes"),
        }));

        self.state = State::ChooseAnAuthScheme {
            key_pair: keys,
            foreign_public_key: host_public_key,
        };

        Ok(None)
    }

    fn handle_auth_scheme(
        &mut self,
        msg: AuthSchemeMessage,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        events.push(InformEvent::WpskkaClientInform(
            WpskkaClientInform::AuthScheme(msg.auth_schemes),
        ));
        Ok(None)
    }

    fn derive_keys(
        key_pair: KeyPair,
        foreign_public_key: [u8; 32],
    ) -> Result<(CipherReliablePeer, CipherUnreliablePeer), ()> {
        let client_public_key = parse_foreign_public(&foreign_public_key);
        let (receive_reliable, send_reliable, receive_unreliable, send_unreliable) =
            diffie_hellman(key_pair.ephemeral_private_key, client_public_key).map_err(|_| ())?;
        // TODO zero hella
        Ok((
            CipherReliablePeer::new(send_reliable.to_vec(), receive_reliable.to_vec()),
            CipherUnreliablePeer::new(send_unreliable.to_vec(), receive_unreliable.to_vec()),
        ))
    }

    fn derive_key_wrapper(
        &mut self,
        key_state: KeyState,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), WpskkaClientError> {
        match Self::derive_keys(key_state.key_pair, key_state.foreign_public_key) {
            Ok((reliable, unreliable)) => {
                self.state = State::Authenticated {
                    reliable,
                    unreliable,
                };
                events.push(InformEvent::WpskkaClientInform(
                    WpskkaClientInform::AuthSuccessful,
                ));
                Ok(())
            }
            Err(()) => {
                self.state = State::KeyExchange;
                events.push(InformEvent::WpskkaClientInform(
                    WpskkaClientInform::AuthFailed,
                ));
                Err(WpskkaClientError::RingError)
            }
        }
    }

    fn handle_auth_message_srp_auth_client(
        &mut self,
        msg: AuthMessage,
        events: &mut Vec<InformEvent>,
        mut srp_client: SrpAuthClient<32>,
        private_key: EphemeralPrivateKey,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        let msg = SrpMessage::read(&mut Cursor::new(&msg.data))
            .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)?;
        match srp_client.handle(msg) {
            Ok(inform) => {
                if let Some(inform) = inform {
                    events.push(InformEvent::WpskkaClientInform(inform));
                }
                if srp_client.is_authenticated() {
                    let (my_pub, their_pub) = srp_client.finish();
                    let keypair = KeyPair {
                        public_key: my_pub,
                        ephemeral_private_key: private_key,
                    };
                    self.derive_key_wrapper(
                        KeyState {
                            key_pair: keypair,
                            foreign_public_key: their_pub,
                        },
                        events,
                    )?;
                    Ok(None)
                } else {
                    self.state = State::IsAuthenticating {
                        auth_scheme: AuthScheme::SrpAuthClient(srp_client),
                        private_key,
                    };
                    Ok(None)
                }
            }
            Err(err) => match err {
                SrpClientError::WrongMessageForState(..) =>
                    Err(WpskkaClientError::BadAuthSchemeMessage),
                _ => {
                    events.push(InformEvent::WpskkaClientInform(
                        WpskkaClientInform::AuthFailed,
                    ));
                    let (public_key, foreign_public_key) = srp_client.finish();

                    let key_pair = KeyPair {
                        public_key,
                        ephemeral_private_key: private_key,
                    };

                    self.state = State::ChooseAnAuthScheme {
                        key_pair,
                        foreign_public_key,
                    };

                    Ok(None)
                }
            },
        }
    }

    fn handle_auth_message(
        &mut self,
        msg: AuthMessage,
        _write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        match mem::replace(&mut self.state, State::Modifying) {
            State::IsAuthenticating {
                auth_scheme,
                private_key,
            } => match auth_scheme {
                AuthScheme::SrpAuthClient(srp_client) =>
                    self.handle_auth_message_srp_auth_client(msg, events, srp_client, private_key),
                _ => {
                    panic!(
                        "Somehow an unsupported auth scheme ended up in the auth scheme. Someone \
                         should get fired."
                    );
                }
            },
            State::Authenticated { .. } => Ok(None),
            _ => Err(WpskkaClientError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            )),
        }
    }

    fn handle_auth_result(
        &mut self,
        msg: AuthResult,
        _write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        if !msg.ok {
            let result = match mem::replace(&mut self.state, State::Modifying) {
                // We are already in the proper state. TODO is this really an error?
                State::ChooseAnAuthScheme { .. } => Ok(None),
                State::IsAuthenticating {
                    auth_scheme,
                    private_key: my_private_key,
                } => {
                    let (public_key, foreign_public_key) = match auth_scheme {
                        AuthScheme::None {
                            public_key,
                            foreign_public_key,
                        } => (public_key, foreign_public_key),
                        AuthScheme::SrpAuthClient(srp_client) => {
                            let (public_key, foreign_public_key) = srp_client.finish();
                            (public_key, foreign_public_key)
                        }
                        _ => {
                            panic!(
                                "Somehow an unsupported auth scheme ended up in the auth scheme. \
                                 Someone should get fired."
                            );
                        }
                    };
                    let key_pair = KeyPair {
                        public_key,
                        ephemeral_private_key: my_private_key,
                    };
                    self.state = State::ChooseAnAuthScheme {
                        key_pair,
                        foreign_public_key,
                    };
                    Ok(None)
                }
                _ => Err(WpskkaClientError::WrongMessageForState(
                    debug(&msg),
                    debug(&self.state),
                )),
            };
            events.push(InformEvent::WpskkaClientInform(
                WpskkaClientInform::AuthFailed,
            ));
            return result;
        }

        // auth was successful

        let key_pair = match mem::replace(&mut self.state, State::Modifying) {
            State::IsAuthenticating {
                auth_scheme,
                private_key,
            } => match auth_scheme {
                AuthScheme::None {
                    public_key,
                    foreign_public_key,
                } => {
                    let key_pair = KeyPair {
                        public_key,
                        ephemeral_private_key: private_key,
                    };
                    Some((key_pair, foreign_public_key))
                }
                // Srp auth is bidirectional so we don't care if the Host auth's us
                AuthScheme::SrpAuthClient(_) => {
                    self.state = State::IsAuthenticating {
                        auth_scheme,
                        private_key,
                    };
                    None
                }
                _ => {
                    todo!("handle auth result for non-srp auth scheme");
                }
            },
            State::Authenticated {
                reliable,
                unreliable,
            } => {
                self.state = State::Authenticated {
                    reliable,
                    unreliable,
                };
                None
            }
            _ =>
                return Err(WpskkaClientError::WrongMessageForState(
                    debug(&msg),
                    debug(&self.state),
                )),
        };

        if let Some((key_pair, foreign_public_key)) = key_pair {
            self.derive_key_wrapper(
                KeyState {
                    key_pair,
                    foreign_public_key,
                },
                events,
            )?;
        }

        Ok(None)
    }

    fn handle_message_reliable(
        &mut self,
        msg: TransportDataMessageReliable<'_>,
        _write: &mut Vec<WpskkaMessage<'_>>,
        _events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        match &mut self.state {
            State::Authenticated { reliable, .. } => Ok(Some(
                reliable
                    .decrypt(msg.data.0.as_ref())
                    .map_err(WpskkaClientError::CipherError)?,
            )),
            _ => Err(WpskkaClientError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            )),
        }
    }

    fn handle_message_unreliable(
        &mut self,
        msg: TransportDataMessageUnreliable<'_>,
        _write: &mut Vec<WpskkaMessage<'_>>,
        _events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        match &mut self.state {
            State::Authenticated { unreliable, .. } => Ok(Some(
                unreliable
                    .decrypt(msg.data.0.as_ref(), msg.counter)
                    .map_err(WpskkaClientError::CipherError)?,
            )),
            _ => Err(WpskkaClientError::WrongMessageForState(
                debug(&msg),
                debug(&self.state),
            )),
        }
    }

    pub fn process_password(
        &mut self,
        password: &[u8],
    ) -> Result<WpskkaMessage<'static>, WpskkaClientError> {
        match &mut self.state {
            State::IsAuthenticating { auth_scheme, .. } => match auth_scheme {
                AuthScheme::SrpAuthClient(srp_client) => srp_client
                    .process_password(password)
                    .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)
                    .and_then(|outgoing| {
                        outgoing
                            .to_bytes()
                            .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)
                    })
                    .map(|data| WpskkaMessage::AuthMessage(AuthMessage { data })),
                _ => Err(WpskkaClientError::WrongAuthScheme(
                    "AuthSchemeMessage::SrpAuthClient",
                    "AuthSchemeMessage::SrpAuthHost",
                )),
            },
            _ => Err(WpskkaClientError::WrongState(
                debug(&self.state),
                "State::Authenticated".to_string(),
            )),
        }
    }

    pub fn try_auth(&mut self, scheme: AuthSchemeType) -> WpskkaMessage<'static> {
        let (key_pair, foreign_public_key) = match mem::replace(&mut self.state, State::Modifying) {
            State::ChooseAnAuthScheme {
                key_pair,
                foreign_public_key,
            } => (key_pair, foreign_public_key),
            State::IsAuthenticating {
                auth_scheme,
                private_key,
            } => {
                let (public_key, foreign_public_key) = match auth_scheme {
                    AuthScheme::None {
                        public_key,
                        foreign_public_key,
                    } => (public_key, foreign_public_key),
                    AuthScheme::SrpAuthClient(srp_client) => srp_client.finish(),
                    _ => panic!("unexpected auth scheme"),
                };
                let key_pair = KeyPair {
                    public_key,
                    ephemeral_private_key: private_key,
                };
                (key_pair, foreign_public_key)
            }
            _ => {
                panic!("try_auth called in a state {:?}", &self.state);
            }
        };

        let auth_scheme = match scheme {
            AuthSchemeType::None => AuthScheme::None {
                public_key: key_pair.public_key,
                foreign_public_key,
            },
            AuthSchemeType::SrpDynamic | AuthSchemeType::SrpStatic => {
                let srp_client = SrpAuthClient::new(key_pair.public_key, foreign_public_key);
                AuthScheme::SrpAuthClient(srp_client)
            }
            AuthSchemeType::PublicKey => {
                todo!("PublicKey")
            }
        };

        self.state = State::IsAuthenticating {
            auth_scheme,
            private_key: key_pair.ephemeral_private_key,
        };
        WpskkaMessage::TryAuth(TryAuth {
            auth_scheme: scheme,
        })
    }
}

impl WpskkaHandlerTrait for WpskkaClientHandler {
    fn handle(
        &mut self,
        msg: WpskkaMessage<'_>,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaError> {
        println!("client received: {:?}", msg);
        let ret = self.handle_internal(msg, write, events).map_err(Into::into);
        assert!(
            !matches!(self.state, State::Modifying),
            "state was left in modifying state"
        );
        println!("client sent: {:?}", write);
        ret
    }

    fn unreliable_cipher(&mut self) -> &mut CipherUnreliablePeer {
        match &mut self.state {
            State::Authenticated { unreliable, .. } => unreliable,
            _ => panic!("Trying to get the unreliable cipher when not authenticated"),
        }
    }

    fn reliable_cipher_mut(&mut self) -> &mut CipherReliablePeer {
        match &mut self.state {
            State::Authenticated { reliable, .. } => reliable,
            _ => panic!("Trying to get the reliable cipher when not authenticated"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaClientError {
    #[error("invalid message {0} for state {1}")]
    WrongMessageForState(String, String),
    #[error("invalid state for function call: got {0} but expected {1}")]
    WrongState(String, String),
    #[error("invalid auth scheme for function call: got {0} but expected {1}")]
    WrongAuthScheme(&'static str, &'static str),

    #[error("unsupported auth scheme type")]
    BadAuthSchemeType(AuthSchemeType),
    #[error("BadAuthSchemeMessage")]
    BadAuthSchemeMessage,

    #[error("ring error")]
    RingError,
    #[error("{0}")]
    CipherError(#[from] CipherError),
}

pub enum WpskkaClientInform {
    AuthScheme(Vec<AuthSchemeType>), // List of available auth schemes
    PasswordPrompt,                  // UI needs to prompt for a password
    AuthFailed,                      // Authentication Failed
    AuthSuccessful,
}
