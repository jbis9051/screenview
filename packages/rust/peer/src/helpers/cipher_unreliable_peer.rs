use super::anti_replay::AntiReplay;
use crate::helpers::cipher_reliable_peer::CipherError;
use common::sel_cipher;

pub struct CipherUnreliablePeer {
    send_key: Vec<u8>,
    send_nonce: u64,
    receive_key: Vec<u8>,
    anti_replay: AntiReplay,
    context: Vec<u8>,
}

impl CipherUnreliablePeer {
    pub fn new(send_key: Vec<u8>, receive_key: Vec<u8>, context: Vec<u8>) -> Self {
        // TODO ensure keys are long enough
        Self {
            send_key,
            send_nonce: 0,
            receive_key,
            anti_replay: AntiReplay::new(),
            context,
        }
    }

    pub fn encrypt(&mut self, plainbytes: &[u8]) -> Result<(Vec<u8>, u64), CipherError> {
        let prev = self.send_nonce;
        self.send_nonce = self
            .send_nonce
            .checked_add(1)
            .ok_or(CipherError::MaximumNonceExceeded("send_nonce"))?;

        let cipherbytes = sel_cipher::encrypt(plainbytes, &self.send_key, &self.context, prev)
            .map_err(|_| CipherError::CipherError)?;

        Ok((cipherbytes, prev))
    }

    pub fn decrypt(&mut self, cipherbytes: &[u8], counter: u64) -> Result<Vec<u8>, CipherError> {
        let is_valid = self.anti_replay.update(counter);

        if !is_valid {
            return Err(CipherError::MessageTooOld(counter));
        }

        let plainbytes =
            sel_cipher::decrypt(cipherbytes, &self.receive_key, &self.context, counter)
                .map_err(|_| CipherError::CipherError)?;
        Ok(plainbytes)
    }
}
