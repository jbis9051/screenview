pub mod auth;
mod client;
mod host;

pub use client::*;
pub use host::*;

use super::helpers::cipher_reliable_peer::CipherError;
use crate::{
    helpers::{
        cipher_reliable_peer::CipherReliablePeer,
        cipher_unreliable_peer::CipherUnreliablePeer,
        crypto::{diffie_hellman, parse_foreign_public, KeyPair},
    },
    InformEvent,
};
use common::messages::{
    wpskka::{TransportDataMessageReliable, TransportDataMessageUnreliable, WpskkaMessage},
    Data,
};
use std::{borrow::Cow, sync::Arc};

// The initial state for the WPSKKA protocol is WpskkaHost::PreInit and WpskkaClient::KeyExchange.
//
// For SRP authentication the flow looks like:
//
// 1. WpskkaHost::set_static_password or WpskkaHost::set_dynamic_password is called to set the password for SRP. This is done before the connection is established.
// 2. WpskkaHost::key_exchange() is called, which generates a keypair, updates the state to KeyExchange, and returns a KeyExchange message.
// 3. Client receives the KeyExchange method, generates it's own keypair, updates the state to ChooseAuthMethod, and sends a KeyExchange message.
// 4. Host receives the KeyExchange method, updates state to PreAuthSelect, sends out available authentication methods
// 5. Client receives the available authentication methods and emits and InformEvent::AuthMethods event with the auth methods.
// 6. WpsskkaClient::try_auth() is called (with srp as the scheme), which updates the state to IsAuthenticating, and returns a TryAuth message.
// 7. Host receives the TryAuth message, updates the state to IsAuthenticating get the proper password, creates an SRP instance, initiates it with the password with srp.init, which returns a message, which the Host sends.
// 8. Client receives a message from Host and emits as an InformEvent::PasswordPrompt event
// 9. Client::process_password() is called called which returns a message
// 10. Host receives the message and if authentication is successful, updates the state to IsAuthenticated, and then sends the last SRP message. The Host also sends a successful AuthResult message.
// 11. The Client ignores the successful authentication result and uses the message from the Host to authenticate the Host. Once authenticated, the Client updates the state to IsAuthenticated.
//
// If authentication fails, the Client emits an InformEvent::AuthFailed and updates the state back to ChooseAuthMethod.  WpsskkaClient::try_auth() should be called again. On the Host side, the state is updated to PreAuthSelect and a InformEvent::AuthFailed is emitted.
// If an error occurs, the error is returned from the handle method and the state is updated to the initial state. It is likely the caller wants to disconnect from the peer.
// It is the callers responsibility to call the methods listed and send what they return.
//
// Note: Step 11 is notably different for other authentication schemes. In other schemes, once the successful auth result is received, the Client immediately updates the state to IsAuthenticated. SRP authentication, however, is bidirectional. That is each party independently authenticates the other. So the Client only cares about authenticating the Host.
//
// "What happens if the Client authenticates the Host but then the Client receives a failed authentication result from the Host?"
// This is guaranteed not to happen as per the spec. The Host MUST send out a successful authentication result after it sends the final SRP message to the Client.


pub struct KeyState {
    foreign_public_key: [u8; 32],
    key_pair: KeyPair,
}

pub trait WpskkaHandlerTrait {
    fn handle(
        &mut self,
        msg: WpskkaMessage<'_>,
        write: &mut Vec<WpskkaMessage<'_>>,
        events: &mut Vec<InformEvent>,
    ) -> Result<Option<Vec<u8>>, WpskkaError>;

    fn unreliable_cipher(&self) -> &Arc<CipherUnreliablePeer>;

    fn wrap_unreliable(
        msg: Vec<u8>,
        cipher: &CipherUnreliablePeer,
    ) -> Result<TransportDataMessageUnreliable<'static>, CipherError> {
        let (data, counter) = cipher.encrypt(&msg)?;
        Ok(TransportDataMessageUnreliable {
            counter,
            data: Data(Cow::Owned(data)),
        })
    }

    fn reliable_cipher_mut(&mut self) -> &mut CipherReliablePeer;

    fn wrap_reliable(
        &mut self,
        msg: Vec<u8>,
    ) -> Result<TransportDataMessageReliable<'static>, CipherError> {
        let cipher = self.reliable_cipher_mut();

        Ok(TransportDataMessageReliable {
            data: Data(Cow::Owned(cipher.encrypt(&msg)?)),
        })
    }
}


#[derive(Debug, thiserror::Error)]
pub enum WpskkaError {
    #[error("host error: {0}")]
    Host(#[from] WpskkaHostError),
    #[error("client error: {0}")]
    Client(#[from] WpskkaClientError),
}
