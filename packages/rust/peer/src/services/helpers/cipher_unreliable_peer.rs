use crate::services::helpers::cipher_reliable_peer::CipherError;
use common::tel_cipher;

pub struct CipherUnreliablePeer {
    send_key: Vec<u8>,
    send_nonce: u64,
    receive_key: Vec<u8>,
    receive_nonce: u64,
}

impl CipherUnreliablePeer {
    pub fn new(send_key: Vec<u8>, receive_key: Vec<u8>) -> Self {
        Self {
            send_key,
            send_nonce: 0,
            receive_key,
            receive_nonce: 0,
        }
    }

    pub fn encrypt(&mut self, plainbytes: Vec<u8>) -> Result<Vec<u8>, CipherError> {
        let cipherbytes = tel_cipher::encrypt(plainbytes, &self.send_key, self.send_nonce)
            .map_err(|_| CipherError::CipherError)?;
        self.send_nonce = self
            .send_nonce
            .checked_add(1)
            .ok_or(CipherError::MaximumNonceExceeded("send_nonce"))?;
        Ok(cipherbytes)
    }

    pub fn decrypt(&mut self, cipherbytes: Vec<u8>, counter: u64) -> Result<Vec<u8>, CipherError> {
        // TODO counter validation https://github.com/WireGuard/wireguard-rs/blob/7d84ef9064559a29b23ab86036f7ef62b450f90c/src/wireguard/router/anti_replay.rs
        let plainbytes = tel_cipher::decrypt(cipherbytes, &self.receive_key, counter)
            .map_err(|_| CipherError::CipherError)?;
        // TODO self.receive_nonce change
        // TODO update replay prevention
        Ok(plainbytes)
    }
}
