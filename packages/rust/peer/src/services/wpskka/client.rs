use crate::services::{
    helpers::{
        cipher_reliable_peer::{CipherError, CipherReliablePeer},
        cipher_unreliable_peer::CipherUnreliablePeer,
        wpskka_common::{hmac, hmac_verify, kdf1, keypair, random_srp_private_value},
    },
    InformEvent,
};
use common::{
    constants::{HashAlgo, SRP_PARAM},
    messages::{
        wpskka::{ClientHello, HostHello, WpskkaMessage},
        ScreenViewMessage,
    },
};
use ring::agreement::{EphemeralPrivateKey, PublicKey};
use srp::{client::SrpClient, types::SrpAuthError};
use std::sync::mpsc::{SendError, Sender};

#[derive(Copy, Clone, Debug)]
pub enum State {
    PreHello,
    WaitingUserInputForPassword,
    PreVerify,
    Data,
}

pub struct WpskkaClientHandler {
    state: State,
    reliable: Option<CipherReliablePeer>,
    unreliable: Option<CipherUnreliablePeer>,
    srp_client: SrpClient<'static, HashAlgo>,
    host_hello: Option<HostHello>,
    keys: Option<(EphemeralPrivateKey, PublicKey)>,
    hmac_key: Option<[u8; 32]>,
}

impl WpskkaClientHandler {
    pub fn new() -> Self {
        Self {
            state: State::PreHello,
            reliable: None,
            unreliable: None,
            srp_client: SrpClient::new(SRP_PARAM),
            host_hello: None,
            keys: None,
            hmac_key: None,
        }
    }

    pub fn process_password(
        &mut self,
        password: &str,
        write: Sender<ScreenViewMessage>,
    ) -> Result<(), WpskkaClientError> {
        // we've received the password form node we need to do some srp stuff, generate our ephemeral keys, and then send a mac to authenticate our keys
        let msg = self.host_hello.as_ref().unwrap();
        let a = random_srp_private_value();
        let verifier = self
            .srp_client
            .process_reply(
                &a,
                &msg.username,
                password.as_bytes(),
                &msg.salt,
                &msg.b_pub,
            )
            .map_err(WpskkaClientError::SrpError)?;
        let a_pub = self.srp_client.compute_public_ephemeral(&a);
        let srp_key = verifier.key();

        let keys = keypair().map_err(|_| WpskkaClientError::RingError)?;

        let srp_key_kdf = kdf1(srp_key);

        let mac = hmac(&srp_key_kdf, keys.1.as_ref());

        write
            .send(ScreenViewMessage::WpskkaMessage(
                WpskkaMessage::ClientHello(ClientHello {
                    a_pub: a_pub.try_into().unwrap(),
                    public_key: keys.1.as_ref().try_into().unwrap(),
                    mac: mac.try_into().unwrap(),
                }),
            ))
            .map_err(WpskkaClientError::WriteError)?;

        // save some stuff we'll need soon
        self.keys = Some(keys);
        self.hmac_key = Some(srp_key_kdf);

        self.state = State::PreVerify;

        Ok(())
    }

    pub fn handle(
        &mut self,
        msg: WpskkaMessage,
        events: Sender<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaClientError> {
        match self.state {
            State::PreHello => match msg {
                WpskkaMessage::HostHello(msg) => {
                    // We need to prompt for user input here, so we dispatch the PasswordPrompt event. Once the user has entered the password someone should call self.process_password
                    self.host_hello = Some(msg);
                    self.state = State::WaitingUserInputForPassword;
                    events
                        .send(InformEvent::WpskkaClientInform(
                            WpskkaClientInform::PasswordPrompt,
                        ))
                        .map_err(WpskkaClientError::InformError)?;
                    Ok(None)
                }
                _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
            },
            State::PreVerify => match msg {
                WpskkaMessage::HostVerify(msg) => {
                    let hmac_key = self.hmac_key.take().unwrap();
                    let host_hello = self.host_hello.take().unwrap();

                    if !hmac_verify(&hmac_key, &host_hello.b_pub, &msg.mac) {
                        return Err(WpskkaClientError::AuthError);
                    }

                    self.state = State::Data;
                    Ok(None)
                }
                _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
            },
            State::Data => match msg {
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
            _ => Err(WpskkaClientError::WrongMessageForState(msg, self.state)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WpskkaClientError {
    #[error("{0}")]
    CipherError(CipherError),
    #[error("{0}")]
    SrpError(SrpAuthError),
    #[error("write error")]
    WriteError(SendError<ScreenViewMessage>),
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(WpskkaMessage, State),
    #[error("inform error")]
    InformError(SendError<InformEvent>),
    #[error("ring error")]
    RingError,
    #[error("authentication error")]
    AuthError,
}

pub enum WpskkaClientInform {
    PasswordPrompt,
}
