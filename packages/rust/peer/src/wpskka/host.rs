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
        AuthScheme as AuthSchemeMessage,
        AuthSchemeType,
        TransportDataMessageReliable,
        TransportDataMessageUnreliable,
        TryAuth,
        WpskkaMessage,
    },
    MessageComponent,
};
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

    keys: Option<Box<KeyPair>>,
    client_public_key: Option<[u8; 32]>,
}

impl Default for WpskkaHostHandler {
    fn default() -> Self {
        Self::new()
    }
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
            client_public_key: None,
        }
    }

    /// Warning: this erases and regenerates keys on every call
    pub fn auth_schemes(&mut self) -> Result<WpskkaMessage, WpskkaHostError> {
        let keys = keypair().map_err(|_| WpskkaHostError::RingError)?;
        let mut auth_schemes = Vec::new();
        if self.static_password.is_some() {
            auth_schemes.push(AuthSchemeType::SrpStatic);
        }
        if self.dynamic_password.is_some() {
            auth_schemes.push(AuthSchemeType::SrpDynamic);
        }
        let msg = WpskkaMessage::AuthScheme(AuthSchemeMessage {
            public_key: keys.public_key.as_ref().try_into().unwrap(),
            auth_schemes,
        });
        self.keys = Some(Box::new(keys));
        Ok(msg)
    }

    pub fn handle_try_auth(
        &mut self,
        msg: TryAuth,
        write: &mut Vec<WpskkaMessage>,
    ) -> Result<(), WpskkaHostError> {
        self.client_public_key = Some(msg.public_key);
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
                self.state = State::IsAuthenticating;

                Ok(())
            }
            AuthSchemeType::PublicKey => Err(WpskkaHostError::BadAuthSchemeType(msg.auth_scheme)),
        }
    }

    pub fn _handle(
        &mut self,
        msg: WpskkaMessage,
        write: &mut Vec<WpskkaMessage>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match self.state {
            State::PreAuthSelect => match msg {
                WpskkaMessage::TryAuth(msg) => {
                    self.handle_try_auth(msg, write)?;
                    Ok(None)
                }
                _ => Err(WpskkaHostError::WrongMessageForState(
                    debug(&msg),
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
                                        events.push(InformEvent::WpskkaHostInform(
                                            WpskkaHostInform::AuthSuccessful,
                                        ));
                                        self.derive_keys()?;
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
                    }
                }
                _ => Err(WpskkaHostError::WrongMessageForState(
                    debug(&msg),
                    self.state,
                )),
            },
            State::Authenticated => match msg {
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
                    self.state,
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

    pub fn authenticated(&self) -> bool {
        matches!(self.state, State::Authenticated)
    }

    /// Warning: Steals keys, Overwrites ciphers
    /// DO NOT CALL THIS FUNCTION WITHOUT AUTHENTICATING THE FOREIGN PUBLIC KEY OR THE WORLD WILL END
    fn derive_keys(&mut self) -> Result<(), WpskkaHostError> {
        // TODO zero data
        let keys = self.keys.take().unwrap();
        let host_pubkey = self.client_public_key.take().unwrap();
        let host_pubkey = parse_foreign_public(&host_pubkey);
        let (send_reliable, receive_reliable, send_unreliable, receive_unreliable) =
            diffie_hellman(keys.ephemeral_private_key, host_pubkey)
                .map_err(|_| WpskkaHostError::RingError)?;
        // TODO zero hella
        self.reliable = Some(CipherReliablePeer::new(
            send_reliable.to_vec(),
            receive_reliable.to_vec(),
        ));
        self.unreliable = Some(Arc::new(CipherUnreliablePeer::new(
            send_unreliable.to_vec(),
            receive_unreliable.to_vec(),
        )));
        Ok(())
    }
}

impl WpskkaHandlerTrait for WpskkaHostHandler {
    fn handle(
        &mut self,
        msg: WpskkaMessage,
        write: &mut Vec<WpskkaMessage>,
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
    #[error("invalid message {0} for state {1:?}")]
    WrongMessageForState(String, State),

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
