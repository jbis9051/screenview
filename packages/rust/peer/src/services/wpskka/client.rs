use crate::services::{
    helpers::{
        cipher_reliable_peer::{CipherError, CipherReliablePeer},
        cipher_unreliable_peer::CipherUnreliablePeer,
        wpskka_common::keypair,
    },
    wpskka::auth::{
        srp_client::{SrpAuthClient, SrpClientError},
        AuthScheme,
    },
    InformEvent,
};
use common::{
    constants::{HashAlgo, SRP_PARAM},
    messages::{
        auth::srp::SrpMessage,
        wpskka::{AuthMessage, AuthSchemeType, TryAuth, WpskkaMessage},
        MessageComponent,
        ScreenViewMessage,
    },
};
use ring::agreement::{EphemeralPrivateKey, PublicKey};
use srp::{client::SrpClient, types::SrpAuthError};
use std::{
    cmp::Ordering,
    collections::HashSet,
    io::Cursor,
    sync::mpsc::{SendError, Sender},
};

#[derive(Copy, Clone, Debug)]
pub enum State {
    WaitingForAuthSchemes,
    IsAuthenticating,
    Authenticated,
}

pub struct WpskkaClientHandler {
    state: State,

    available_auth_schemes: Vec<AuthSchemeType>,
    current_auth_num: usize,
    current_auth: Option<AuthScheme>,
    password: Option<Vec<u8>>,

    reliable: Option<CipherReliablePeer>,
    unreliable: Option<CipherUnreliablePeer>,

    keys: Option<(EphemeralPrivateKey, PublicKey)>,
    host_public_key: Option<[u8; 16]>,
}

impl WpskkaClientHandler {
    pub fn new() -> Self {
        Self {
            state: State::WaitingForAuthSchemes,
            available_auth_schemes: Default::default(),
            current_auth_num: 0,
            current_auth: None,
            password: None,
            reliable: None,
            unreliable: None,
            keys: None,
            host_public_key: None,
        }
    }

    pub fn process_password(
        &mut self,
        password: &[u8],
        write: Sender<ScreenViewMessage>,
    ) -> Result<(), WpskkaClientError> {
        self.password = Some(password.to_vec());
        if let Some(auth) = self.current_auth.as_mut() {
            return match auth {
                AuthScheme::SrpAuthClient(client) => {
                    let outgoing = client
                        .process_password(password)
                        .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)?;
                    write
                        .send(ScreenViewMessage::WpskkaMessage(
                            WpskkaMessage::AuthMessage(AuthMessage {
                                data: outgoing
                                    .to_bytes()
                                    .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)?,
                            }),
                        ))
                        .map_err(WpskkaClientError::WriteError)?;
                    Ok(())
                }
                _ => Ok(()),
            };
        }
        Ok(())
    }

    pub fn next_auth(
        &mut self,
        write: Sender<ScreenViewMessage>,
        events: Sender<InformEvent>,
    ) -> Result<(), WpskkaClientError> {
        while self.current_auth_num < self.available_auth_schemes.len() {
            let current_auth = self.available_auth_schemes[self.current_auth_num];
            self.current_auth_num += 1;

            // SrpStatic and SrpDynamic are the same (Srp) so lets handle them together
            if current_auth == AuthSchemeType::SrpStatic
                || current_auth == AuthSchemeType::SrpDynamic
            {
                self.current_auth = Some(AuthScheme::SrpAuthClient(SrpAuthClient::new(
                    self.keys.as_ref().unwrap().1.clone(),
                )));
                write
                    .send(ScreenViewMessage::WpskkaMessage(WpskkaMessage::TryAuth(
                        TryAuth {
                            public_key: self.keys.as_ref().unwrap().1.as_ref().try_into().unwrap(), // send our public key
                            auth_scheme: current_auth,
                        },
                    )))
                    .map_err(WpskkaClientError::WriteError)?;
                self.state = State::IsAuthenticating;
                return Ok(());
            }

            // TODO support other auth methods
        }
        events
            .send(InformEvent::WpskkaClientInform(
                WpskkaClientInform::OutOfAuthenticationSchemes,
            ))
            .map_err(WpskkaClientError::InformError)?;
        Ok(())
    }

    pub fn handle(
        &mut self,
        msg: WpskkaMessage,
        write: Sender<ScreenViewMessage>,
        events: Sender<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        match self.state {
            State::WaitingForAuthSchemes => match msg {
                WpskkaMessage::AuthScheme(msg) => {
                    self.available_auth_schemes = msg.num_auth_schemes;
                    self.available_auth_schemes.dedup();
                    self.available_auth_schemes.sort_by(|a, b| {
                        let a_is_srp =
                            (a == &AuthSchemeType::SrpStatic || a == &AuthSchemeType::SrpDynamic);
                        let b_is_srp =
                            (b == &AuthSchemeType::SrpStatic || b == &AuthSchemeType::SrpDynamic);
                        if a_is_srp && !b_is_srp {
                            return Ordering::Greater;
                        }
                        if !a_is_srp && b_is_srp {
                            return Ordering::Less;
                        }
                        Ordering::Equal
                    }); // Put SrpStatic and SrpDynamic in front
                    self.host_public_key = Some(msg.public_key);
                    self.keys = Some(keypair().map_err(|_| WpskkaClientError::RingError)?);
                    self.next_auth(write, events)?;
                    Ok(None)
                }
                _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
            },
            State::IsAuthenticating => match msg {
                WpskkaMessage::AuthMessage(msg) => {
                    return match self.current_auth.as_mut().unwrap() {
                        AuthScheme::SrpAuthClient(srp_client) => {
                            let msg = SrpMessage::read(&mut Cursor::new(&msg.data))
                                .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)?;
                            match srp_client.handle(msg) {
                                Ok(outgoing) => {
                                    if let Some(outgoing) = outgoing {
                                        events
                                            .send(InformEvent::WpskkaClientInform(outgoing))
                                            .map_err(WpskkaClientError::InformError)?;
                                    }
                                    if srp_client.is_authenticated() {
                                        self.current_auth = None; // TODO Security: Look into zeroing out the data here
                                        self.state = State::Authenticated;
                                    }
                                    Ok(None)
                                }
                                Err(err) => {
                                    match err {
                                        SrpClientError::WrongMessageForState(..) => {
                                            return Err(WpskkaClientError::BadAuthSchemeMessage);
                                        }
                                        _ => {
                                            self.current_auth = None; // TODO Security: Look into zeroing out the data here
                                            self.next_auth(write, events)?;
                                            Ok(None)
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            panic!(
                                "Somehow an unsupported auth scheme ended up in the auth scheme. \
                                 Someone should get fired."
                            )
                        }
                    };
                }
                WpskkaMessage::AuthResult(msg) => {
                    if !msg.ok {
                        self.current_auth = None; // TODO Security: Look into zeroing out the data here
                        self.next_auth(write, events)?;
                    }
                    Ok(None)
                    // We don't really care whether we are authenticated to the Host, we care if the Host is authenticated to us. We know this by asking the self.current_auth.is_authenticated
                    // TODO maybe check if we are authenticated here. We shouldn't need it for SRP but in future, other auth methods may
                }
                _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
            },
            State::Authenticated => match msg {
                WpskkaMessage::AuthResult(msg) => {
                    if !msg.ok {
                        // We authenticated the Host but the Host couldn't authenticate us back. Let's try the next auth method.
                        self.current_auth = None; // TODO Security: Look into zeroing out the data here
                        self.state = State::IsAuthenticating;
                        self.next_auth(write, events)?;
                    }
                    Ok(None)
                }
                WpskkaMessage::TransportDataMessageReliable(msg) => {
                    let reliable = self.reliable.as_mut().unwrap();
                    Ok(Some(
                        reliable
                            .decrypt(msg.data)
                            .map_err(WpskkaClientError::CipherError)?,
                    ))
                }
                WpskkaMessage::TransportDataMessageUnreliable(msg) => {
                    let unreliable = self.unreliable.as_mut().unwrap();
                    Ok(Some(
                        unreliable
                            .decrypt(msg.data, msg.counter)
                            .map_err(WpskkaClientError::CipherError)?,
                    ))
                }
                _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaClientError {
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(WpskkaMessage, State),

    #[error("unsupported auth scheme type")]
    BadAuthSchemeType(AuthSchemeType),
    #[error("BadAuthSchemeMessage")]
    BadAuthSchemeMessage,

    #[error("inform error")]
    InformError(SendError<InformEvent>),
    #[error("write error")]
    WriteError(SendError<ScreenViewMessage>),

    #[error("ring error")]
    RingError,
    #[error("{0}")]
    CipherError(CipherError),
}

pub enum WpskkaClientInform {
    PasswordPrompt,
    OutOfAuthenticationSchemes,
    AuthFailed,
}
