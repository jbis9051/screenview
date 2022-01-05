use common::tel_cipher;

pub struct TelCipherReliablePeer {
    send_key: Vec<u8>,
    send_nonce: u64,
    receive_key: Vec<u8>,
    receive_nonce: u64,
}

impl TelCipherReliablePeer {
    pub fn new(send_key: Vec<u8>, receive_key: Vec<u8>) -> Self {
        Self {
            send_key,
            send_nonce: 0,
            receive_key,
            receive_nonce: 0,
        }
    }

    pub fn encrypt(&mut self, plainbytes: Vec<u8>) -> Result<Vec<u8>, TelCipherError> {
        let cipherbytes = tel_cipher::encrypt(plainbytes, &self.send_key, self.send_nonce)
            .map_err(|_| TelCipherError::CipherError)?;
        self.send_nonce = self
            .send_nonce
            .checked_add(1)
            .ok_or(TelCipherError::MaximumNonceExceeded("send_nonce"))?;
        Ok(cipherbytes)
    }

    pub fn decrypt(&mut self, cipherbytes: Vec<u8>) -> Result<Vec<u8>, TelCipherError> {
        let plainbytes = tel_cipher::decrypt(cipherbytes, &self.receive_key, self.receive_nonce)
            .map_err(|_| TelCipherError::CipherError)?;
        self.receive_nonce = self
            .receive_nonce
            .checked_add(1)
            .ok_or(TelCipherError::MaximumNonceExceeded("receives_nonce"))?;
        Ok(plainbytes)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TelCipherError {
    #[error("a cipher error occurred")]
    CipherError,
    #[error("nonce wrap {0}")]
    MaximumNonceExceeded(&'static str),
}
