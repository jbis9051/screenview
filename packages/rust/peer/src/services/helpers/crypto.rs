use common::constants::{Hkdf, Hmac, Mac, SRP_PARAM};
use ring::{
    agreement,
    agreement::{EphemeralPrivateKey, PublicKey},
    error,
    rand,
    rand::{SecureRandom, SystemRandom},
};

pub struct KeyPair {
    pub public_key: PublicKey,
    pub ephemeral_private_key: EphemeralPrivateKey,
}

pub fn random_bytes(bytes: usize) -> Vec<u8> {
    let mut vec = vec![0u8; bytes];
    let rng = SystemRandom::new();
    rng.fill(&mut vec).unwrap();
    vec
}

pub fn random_srp_private_value() -> Vec<u8> {
    random_bytes((SRP_PARAM.n.bits() / 8) as usize)
}

pub fn keypair() -> Result<KeyPair, error::Unspecified> {
    let rng = rand::SystemRandom::new();
    let my_private_key = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng)?;
    let my_public_key = my_private_key.compute_public_key()?;
    Ok(KeyPair {
        public_key: my_public_key,
        ephemeral_private_key: my_private_key,
    })
}

pub fn kdf1(ikm: &[u8]) -> [u8; 32] {
    let kdf = Hkdf::new(None, ikm);
    let mut key = [0u8; 32];
    kdf.expand(&[], &mut key).unwrap();
    key
}

pub fn kdf2(ikm: &[u8]) -> ([u8; 32], [u8; 32]) {
    let kdf = Hkdf::new(None, ikm);
    let mut key = [0u8; 64];
    kdf.expand(&[], &mut key).unwrap();
    let keys = key.split_at(32);
    // 64 / 2 = 32
    (keys.0.try_into().unwrap(), keys.1.try_into().unwrap())
}


#[macro_export]
macro_rules! hash {
    ($($plaintext:expr),*) => {{
        let mut h = common::constants::HashAlgo::new();
        $(
          h.update($plaintext);
        )*
        h.finalize().as_bytes()
    }};
}

pub fn hmac(key: &[u8], input: &[u8]) -> Vec<u8> {
    let mut hmac = Hmac::new_from_slice(key).unwrap();
    hmac.update(input);
    hmac.finalize().into_bytes().to_vec()
}

pub fn hmac_verify(key: &[u8], input: &[u8], verify: &[u8]) -> bool {
    let mut hmac = Hmac::new_from_slice(key).unwrap();
    hmac.update(input);
    hmac.verify_slice(verify).is_ok()
}
