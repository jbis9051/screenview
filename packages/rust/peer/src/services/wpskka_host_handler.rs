use crate::services::helpers::cipher_reliable_peer::{CipherError, CipherReliablePeer};
use crate::services::helpers::cipher_unreliable_peer::CipherUnreliablePeer;
use crate::services::helpers::wpskka_common::{
    hmac, hmac_verify, kdf1, keypair, random_bytes, random_srp_private_value,
};
use common::constants::{HashAlgo, SRP_PARAM};
use common::messages::wpskka::{HostHello, HostVerify, WpskkaMessage};
use common::messages::ScreenViewMessage;
use ring::agreement::{EphemeralPrivateKey, PublicKey};
use srp::client::SrpClient;
use srp::server::SrpServer;
use srp::types::SrpAuthError;
use std::sync::mpsc::{SendError, Sender};

#[derive(Copy, Clone, Debug)]
pub enum State {
    Handshake,
    Data,
}

pub struct WpskkaHostHandler {
    state: State,

    reliable: Option<CipherReliablePeer>,
    unreliable: Option<CipherUnreliablePeer>,

    srp_server: SrpServer<'static, HashAlgo>,
    verifier: Option<Vec<u8>>,
    keys: Option<(EphemeralPrivateKey, PublicKey)>,
    b: Option<Vec<u8>>,
}

impl WpskkaHostHandler {
    pub fn new() -> Self {
        Self {
            state: State::Handshake,
            reliable: None,
            unreliable: None,
            srp_server: SrpServer::new(SRP_PARAM),
            verifier: None,
            keys: None,
            b: None,
        }
    }

    pub fn init(
        &mut self,
        password: &[u8],
        write: &mut Sender<ScreenViewMessage>,
    ) -> Result<(), WpskkaHostError> {
        // For SRP is an asymmetric or Augmented PAKE but we wanted a Balanced PAKE, so Host acts as the Client for registration and the server for authentication
        let client = SrpClient::<'static, HashAlgo>::new(SRP_PARAM);
        let username = random_bytes(16);
        let salt = random_bytes(16);
        let verifier = client.compute_verifier(&username, password, &salt);

        let b = random_srp_private_value();
        let b_pub = self.srp_server.compute_public_ephemeral(&b, &verifier);
        let keys = keypair().map_err(|_| WpskkaHostError::RingError)?;
        write
            .send(ScreenViewMessage::WpskkaMessage(WpskkaMessage::HostHello(
                HostHello {
                    username: username.try_into().unwrap(),
                    salt: salt.try_into().unwrap(),
                    b_pub: b_pub.try_into().unwrap(),
                    public_key: keys.1.as_ref().try_into().unwrap(),
                },
            )))
            .map_err(WpskkaHostError::WriteError)?;

        self.verifier = Some(verifier);
        self.keys = Some(keys);
        self.b = Some(b);

        Ok(())
    }

    pub fn handle(
        &mut self,
        msg: WpskkaMessage,
        write: &mut Sender<ScreenViewMessage>,
    ) -> Result<Option<Vec<u8>>, WpskkaHostError> {
        match self.state {
            State::Handshake => match msg {
                WpskkaMessage::ClientHello(msg) => {
                    let srp_key_kdf = {
                        let srp_verifier = self
                            .srp_server
                            .process_reply(
                                self.b.as_ref().unwrap(),
                                self.verifier.as_ref().unwrap(),
                                &msg.a_pub,
                            )
                            .map_err(WpskkaHostError::SrpError)?;
                        kdf1(srp_verifier.key())
                    };

                    if !hmac_verify(&srp_key_kdf, &msg.public_key, &msg.mac) {
                        return Err(WpskkaHostError::AuthError);
                    }

                    let mac = hmac(&srp_key_kdf, self.keys.as_ref().unwrap().1.as_ref());

                    write
                        .send(ScreenViewMessage::WpskkaMessage(WpskkaMessage::HostVerify(
                            HostVerify {
                                mac: mac.try_into().unwrap(),
                            },
                        )))
                        .map_err(WpskkaHostError::WriteError)?;

                    self.state = State::Data;
                    Ok(None)
                }
                _ => Err(WpskkaHostError::WrongMessageForState(msg, self.state)),
            },
            State::Data => match msg {
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
    #[error("{0}")]
    CipherError(CipherError),
    #[error("{0}")]
    SrpError(SrpAuthError),
    #[error("invalid message {0:?} for state {1:?}")]
    WrongMessageForState(WpskkaMessage, State),
    #[error("ring error")]
    RingError,
    #[error("inform error")]
    InformError(SendError<ScreenViewMessage>),
    #[error("authentication error")]
    AuthError,
    #[error("write error")]
    WriteError(SendError<ScreenViewMessage>),
}
