use crate::{
    debug,
    services::{
        helpers::crypto::{hmac, hmac_verify, kdf1, random_bytes, random_srp_private_value},
        wpskka::auth::srp_host::State::Done,
    },
};
use common::{
    constants::{HashAlgo, SRP_PARAM},
    messages::auth::srp::{HostHello, HostVerify, SrpMessage},
};
use ring::agreement::PublicKey;
use srp::{client::SrpClient, server::SrpServer, types::SrpAuthError};

#[derive(Debug, Copy, Clone)]
pub enum State {
    PreHostHello,
    WaitingForClientHello,
    Done,
}


// Arbitrary SrpAuthHost. Can be used for any SRP based auth scheme.
pub struct SrpAuthHost {
    state: State,
    authenticated: bool,
    verifier: Option<Vec<u8>>,
    client_public_key: Vec<u8>,
    our_public_key: PublicKey,
    b: Option<Vec<u8>>,
}

impl SrpAuthHost {
    pub fn new(our_public_key: PublicKey, client_public_key: Vec<u8>) -> Self {
        Self {
            state: State::PreHostHello,
            authenticated: false,
            verifier: None,
            client_public_key,
            our_public_key,
            b: None,
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    pub fn init(&mut self, password: &[u8]) -> SrpMessage {
        // For SRP is an asymmetric or Augmented PAKE but we wanted a Balanced PAKE, so Host acts as the Client for registration and the server for authentication
        let client = SrpClient::<'static, HashAlgo>::new(SRP_PARAM);
        let username = random_bytes(16);
        let salt = random_bytes(16);
        let verifier = client.compute_verifier(&username, password, &salt);

        let b = random_srp_private_value();
        let srp_server = SrpServer::<'static, HashAlgo>::new(SRP_PARAM);
        let b_pub = srp_server.compute_public_ephemeral(&b, &verifier);

        self.verifier = Some(verifier);
        self.b = Some(b);
        self.state = State::WaitingForClientHello;

        SrpMessage::HostHello(HostHello {
            username: username.try_into().unwrap(),
            salt: salt.try_into().unwrap(),
            b_pub: b_pub.try_into().map(Box::new).unwrap(),
        })
    }

    pub fn handle(&mut self, msg: SrpMessage) -> Result<SrpMessage, SrpHostError> {
        match self.state {
            State::WaitingForClientHello => match msg {
                SrpMessage::ClientHello(msg) => {
                    let srp_server = SrpServer::<'static, HashAlgo>::new(SRP_PARAM);

                    let srp_key_kdf = {
                        let srp_verifier = srp_server
                            .process_reply(
                                &self.b.take().unwrap(),
                                &self.verifier.take().unwrap(),
                                &*msg.a_pub,
                            )
                            .map_err(SrpHostError::SrpAuthError)?;
                        kdf1(srp_verifier.key())
                    };

                    if !hmac_verify(&srp_key_kdf, &self.client_public_key, &msg.mac) {
                        return Err(SrpHostError::AuthFailed);
                    }

                    let mac = hmac(&srp_key_kdf, self.our_public_key.as_ref());

                    self.state = Done;
                    self.authenticated = true;

                    Ok(SrpMessage::HostVerify(HostVerify {
                        mac: mac.try_into().unwrap(),
                    }))
                }
                _ => Err(SrpHostError::WrongMessageForState(debug(&msg), self.state)),
            },
            _ => Err(SrpHostError::WrongMessageForState(debug(&msg), self.state)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SrpHostError {
    #[error("{0}")]
    SrpAuthError(SrpAuthError),
    #[error("auth failed")]
    AuthFailed,
    #[error("invalid message {0} for state {1:?}")]
    WrongMessageForState(String, State),
}
