use chacha20poly1305::{
    aead::{Aead, Error, NewAead},
    ChaCha20Poly1305,
    Key,
    Nonce,
};

fn pad_nonce(nonce: u64) -> [u8; 12] {
    let mut output = [0; 12];
    let bytes = nonce.to_le_bytes();
    (&mut output[4 ..]).copy_from_slice(&bytes);
    output
}

pub fn decrypt(data: &[u8], key: &[u8], nonce: u64) -> Result<Vec<u8>, Error> {
    assert_eq!(key.len(), 32);
    let key = Key::from_slice(key); // 32-bytes
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = pad_nonce(nonce);
    let nonce = Nonce::from_slice(&nonce);
    cipher.decrypt(nonce, data)
}

pub fn encrypt(data: &[u8], key: &[u8], nonce: u64) -> Result<Vec<u8>, Error> {
    assert_eq!(key.len(), 32);
    let key = Key::from_slice(key); // 32-bytes
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = pad_nonce(nonce);
    let nonce = Nonce::from_slice(&nonce);
    cipher.encrypt(nonce, data)
}
