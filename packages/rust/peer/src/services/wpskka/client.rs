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
    SendError,
};
use common::messages::{
    auth::srp::SrpMessage,
    wpskka::{AuthMessage, AuthSchemeType, TryAuth, WpskkaMessage},
    MessageComponent,
    ScreenViewMessage,
};
use ring::agreement::{EphemeralPrivateKey, PublicKey};
use std::{cmp::Ordering, io::Cursor, sync::Arc};

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
    current_auth: Option<Box<AuthScheme>>,
    password: Option<Vec<u8>>,

    reliable: Option<CipherReliablePeer>,
    unreliable: Option<Arc<CipherUnreliablePeer>>,

    keys: Option<Box<(EphemeralPrivateKey, PublicKey)>>,
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

    pub fn reliable_cipher_mut(&mut self) -> &mut CipherReliablePeer {
        self.reliable.as_mut().unwrap()
    }

    pub fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer> {
        self.unreliable.as_ref().unwrap()
    }

    pub fn process_password<F>(
        &mut self,
        password: &[u8],
        write: F,
    ) -> Result<(), WpskkaClientError>
    where
        F: Fn(ScreenViewMessage) -> Result<(), SendError>,
    {
        self.password = Some(password.to_vec());
        if let Some(auth) = self.current_auth.as_mut() {
            return match &mut **auth {
                AuthScheme::SrpAuthClient(client) => {
                    let outgoing = client
                        .process_password(password)
                        .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)?;
                    write(ScreenViewMessage::WpskkaMessage(
                        WpskkaMessage::AuthMessage(AuthMessage {
                            data: outgoing
                                .to_bytes()
                                .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)?,
                        }),
                    ))?;
                    Ok(())
                }
                _ => Ok(()),
            };
        }
        Ok(())
    }

    pub fn next_auth<F>(
        &mut self,
        write: F,
        events: &mut Vec<InformEvent>,
    ) -> Result<(), WpskkaClientError>
    where
        F: Fn(WpskkaMessage) -> Result<(), SendError>,
    {
        while self.current_auth_num < self.available_auth_schemes.len() {
            let current_auth = self.available_auth_schemes[self.current_auth_num];
            self.current_auth_num += 1;

            // SrpStatic and SrpDynamic are the same (Srp) so lets handle them together
            if current_auth == AuthSchemeType::SrpStatic
                || current_auth == AuthSchemeType::SrpDynamic
            {
                self.current_auth = Some(Box::new(AuthScheme::SrpAuthClient(SrpAuthClient::new(
                    self.keys.as_ref().unwrap().1.clone(),
                ))));
                write(WpskkaMessage::TryAuth(TryAuth {
                    public_key: self.keys.as_ref().unwrap().1.as_ref().try_into().unwrap(), // send our public key
                    auth_scheme: current_auth,
                }))?;
                self.state = State::IsAuthenticating;
                return Ok(());
            }

            // TODO support other auth methods
        }
        events.push(InformEvent::WpskkaClientInform(
            WpskkaClientInform::OutOfAuthenticationSchemes,
        ));
        Ok(())
    }

    pub fn handle<F>(
        &mut self,
        msg: WpskkaMessage,
        write: F,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError>
    where
        F: Fn(WpskkaMessage) -> Result<(), SendError>,
    {
        match self.state {
            State::WaitingForAuthSchemes => match msg {
                WpskkaMessage::AuthScheme(msg) => {
                    self.available_auth_schemes = msg.num_auth_schemes;
                    self.available_auth_schemes.dedup();
                    self.available_auth_schemes.sort_by(|a, b| {
                        let a_is_srp =
                            a == &AuthSchemeType::SrpStatic || a == &AuthSchemeType::SrpDynamic;
                        let b_is_srp =
                            b == &AuthSchemeType::SrpStatic || b == &AuthSchemeType::SrpDynamic;
                        if a_is_srp && !b_is_srp {
                            return Ordering::Greater;
                        }
                        if !a_is_srp && b_is_srp {
                            return Ordering::Less;
                        }
                        Ordering::Equal
                    }); // Put SrpStatic and SrpDynamic in front
                    self.host_public_key = Some(msg.public_key);
                    self.keys = Some(Box::new(
                        keypair().map_err(|_| WpskkaClientError::RingError)?,
                    ));
                    self.next_auth(write, events)?;
                    Ok(None)
                }
                _ => Err(WpskkaClientError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
            State::IsAuthenticating => match msg {
                WpskkaMessage::AuthMessage(msg) => {
                    return match &mut **self.current_auth.as_mut().unwrap() {
                        AuthScheme::SrpAuthClient(srp_client) => {
                            let msg = SrpMessage::read(&mut Cursor::new(&msg.data))
                                .map_err(|_| WpskkaClientError::BadAuthSchemeMessage)?;
                            match srp_client.handle(msg) {
                                Ok(outgoing) => {
                                    if let Some(outgoing) = outgoing {
                                        events.push(InformEvent::WpskkaClientInform(outgoing));
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
                _ => Err(WpskkaClientError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
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
                _ => Err(WpskkaClientError::WrongMessageForState(
                    Box::new(msg),
                    self.state,
                )),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaClientError {
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

pub enum WpskkaClientInform {
    PasswordPrompt,
    OutOfAuthenticationSchemes,
    AuthFailed,
}