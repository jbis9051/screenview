use crate::{
    debug,
    helpers::{
        crypto::{hmac, hmac_verify, kdf1, random_srp_private_value},
        left_pad::left_pad,
    },
    wpskka::WpskkaClientInform,
};
use common::{
    constants::{HashAlgo, SRP_PARAM},
    messages::auth::srp::{ClientHello, HostHello, SrpMessage},
};
use ring::agreement::PublicKey;
use srp::{client::SrpClient, types::SrpAuthError};

#[derive(Debug, Copy, Clone)]
pub enum State {
    PreHello,
    WaitingUserInputForPassword,
    PreVerify,
    Done,
}


// Arbitrary SrpAuthClient. Can be used for any SRP based auth scheme.
#[derive(Debug)]
pub struct SrpAuthClient<const N: usize> {
    state: State,
    authenticated: bool,
    host_public_key: [u8; N],
    our_public_key: PublicKey,
    host_hello: Option<Box<HostHello>>,
    hmac_key: Option<Box<[u8; 32]>>,
}

impl<const N: usize> SrpAuthClient<N> {
    pub fn new(our_public_key: PublicKey, host_public_key: [u8; N]) -> Self {
        Self {
            state: State::PreHello,
            authenticated: false,
            host_public_key,
            our_public_key,
            host_hello: None,
            hmac_key: None,
        }
    }

    pub fn finish(self) -> (PublicKey, [u8; N]) {
        (self.our_public_key, self.host_public_key)
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    pub fn process_password(&mut self, password: &[u8]) -> Result<SrpMessage, SrpClientError> {
        // we've received the password form node we need to do some srp stuff and then send a mac to authenticate our keys
        let msg = self.host_hello.take().unwrap();
        let a = random_srp_private_value();
        let srp_client = SrpClient::<'static, HashAlgo>::new(SRP_PARAM);
        let verifier = srp_client
            .process_reply(&a, &msg.username, password, &msg.salt, &*msg.b_pub)
            .map_err(SrpClientError::SrpAuthError)?;
        let a_pub = left_pad(&srp_client.compute_public_ephemeral(&a), 256);

        let srp_key = verifier.key();

        let srp_key_kdf = kdf1(srp_key);

        let mac = hmac(&srp_key_kdf, self.our_public_key.as_ref());

        // save some stuff we'll need soon
        self.hmac_key = Some(Box::new(srp_key_kdf));
        self.state = State::PreVerify;


        Ok(SrpMessage::ClientHello(ClientHello {
            a_pub: a_pub.try_into().map(Box::new).unwrap(), // TODO this may fail cause math
            mac: mac.try_into().unwrap(),
        }))
    }

    pub fn handle(
        &mut self,
        msg: SrpMessage,
    ) -> Result<Option<WpskkaClientInform>, SrpClientError> {
        match self.state {
            State::PreHello => match msg {
                SrpMessage::HostHello(msg) => {
                    // We need to prompt for user input here, so we dispatch the PasswordPrompt event. Once the user has entered the password someone should call self.process_password
                    self.host_hello = Some(Box::new(msg));
                    self.state = State::WaitingUserInputForPassword;
                    Ok(Some(WpskkaClientInform::PasswordPrompt))
                }
                _ => Err(SrpClientError::WrongMessageForState(
                    debug(&msg),
                    self.state,
                )),
            },
            State::PreVerify => match msg {
                SrpMessage::HostVerify(msg) => {
                    let hmac_key = self.hmac_key.take().unwrap();

                    if !hmac_verify(&*hmac_key, self.host_public_key.as_ref(), &msg.mac) {
                        return Err(SrpClientError::AuthFailed);
                    }
                    self.authenticated = true;
                    self.state = State::Done;
                    Ok(None)
                }
                _ => Err(SrpClientError::WrongMessageForState(
                    debug(&msg),
                    self.state,
                )),
            },
            _ => Err(SrpClientError::WrongMessageForState(
                debug(&msg),
                self.state,
            )),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SrpClientError {
    #[error("{0}")]
    SrpAuthError(SrpAuthError),
    #[error("auth failed")]
    AuthFailed,
    #[error("invalid message {0} for state {1:?}")]
    WrongMessageForState(String, State),
}
