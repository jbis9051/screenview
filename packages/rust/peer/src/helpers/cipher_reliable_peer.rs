use common::sel_cipher;

pub struct CipherReliablePeer {
    send_key: Vec<u8>,
    send_nonce: u64,
    receive_key: Vec<u8>,
    receive_nonce: u64,
}

impl CipherReliablePeer {
    pub fn new(send_key: Vec<u8>, receive_key: Vec<u8>) -> Self {
        Self {
            send_key,
            send_nonce: 0,
            receive_key,
            receive_nonce: 0,
        }
    }

    pub fn encrypt(&mut self, plainbytes: &[u8]) -> Result<Vec<u8>, CipherError> {
        let cipherbytes = sel_cipher::encrypt(plainbytes, &self.send_key, self.send_nonce)
            .map_err(|_| CipherError::CipherError)?;
        self.send_nonce = self
            .send_nonce
            .checked_add(1)
            .ok_or(CipherError::MaximumNonceExceeded("send_nonce"))?;
        Ok(cipherbytes)
    }

    pub fn decrypt(&mut self, cipherbytes: &[u8]) -> Result<Vec<u8>, CipherError> {
        let plainbytes = sel_cipher::decrypt(cipherbytes, &self.receive_key, self.receive_nonce)
            .map_err(|_| CipherError::CipherError)?;
        self.receive_nonce = self
            .receive_nonce
            .checked_add(1)
            .ok_or(CipherError::MaximumNonceExceeded("receives_nonce"))?;
        Ok(plainbytes)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("a cipher error occurred")]
    CipherError,
    #[error("nonce wrap {0}")]
    MaximumNonceExceeded(&'static str),
    #[error("the counter ({0}) is too old ")]
    MessageTooOld(u64),
}
