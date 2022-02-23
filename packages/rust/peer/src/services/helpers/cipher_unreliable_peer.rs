use super::{anti_replay::AntiReplay, MAX_NONCE};
use crate::services::helpers::cipher_reliable_peer::CipherError;
use common::sel_cipher;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

pub struct CipherUnreliablePeer {
    send_key: Vec<u8>,
    send_nonce: AtomicU64,
    receive_key: Vec<u8>,
    anti_replay: Mutex<AntiReplay>,
}

impl CipherUnreliablePeer {
    pub fn new(send_key: Vec<u8>, receive_key: Vec<u8>) -> Self {
        // TODO ensure keys are long enough
        Self {
            send_key,
            send_nonce: AtomicU64::new(0),
            receive_key,
            anti_replay: Mutex::new(AntiReplay::new()),
        }
    }

    pub fn encrypt(&self, plainbytes: &[u8]) -> Result<(Vec<u8>, u64), CipherError> {
        // TODO: choose more lenient memory ordering if possible
        let prev = self.send_nonce.fetch_add(1, Ordering::SeqCst);

        // Conservative guard against nonce wrapping
        if prev >= MAX_NONCE {
            return Err(CipherError::MaximumNonceExceeded("send_nonce"));
        }

        let cipherbytes = sel_cipher::encrypt(plainbytes, &self.send_key, prev)
            .map_err(|_| CipherError::CipherError)?;

        Ok((cipherbytes, prev))
    }

    pub fn decrypt(&self, cipherbytes: &[u8], counter: u64) -> Result<Vec<u8>, CipherError> {
        let is_valid = self.anti_replay.lock().unwrap().update(counter);

        if !is_valid {
            return Err(CipherError::MessageTooOld(counter));
        }

        // TODO counter validation https://github.com/WireGuard/wireguard-rs/blob/7d84ef9064559a29b23ab86036f7ef62b450f90c/src/wireguard/router/anti_replay.rs
        let plainbytes = sel_cipher::decrypt(cipherbytes, &self.receive_key, counter)
            .map_err(|_| CipherError::CipherError)?;
        // TODO self.receive_nonce change
        Ok(plainbytes)
    }
}
