use common::constants::{Hkdf, Hmac, Mac, SRP_PARAM};
use ring::{
    agreement,
    agreement::{EphemeralPrivateKey, PublicKey},
    error,
    rand,
    rand::{SecureRandom, SystemRandom},
};

pub fn random_bytes(bytes: usize) -> Vec<u8> {
    let mut vec = vec![0u8; bytes];
    let rng = SystemRandom::new();
    rng.fill(&mut vec).unwrap();
    vec
}

pub fn random_srp_private_value() -> Vec<u8> {
    random_bytes((SRP_PARAM.n.bits() / 8) as usize)
}

pub fn keypair() -> Result<(EphemeralPrivateKey, PublicKey), error::Unspecified> {
    let rng = rand::SystemRandom::new();
    let my_private_key = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng)?;
    let my_public_key = my_private_key.compute_public_key()?;
    Ok((my_private_key, my_public_key))
}

pub fn kdf1(ikm: &[u8]) -> [u8; 32] {
    let kdf = Hkdf::new(None, ikm);
    let mut key = [0u8; 32];
    kdf.expand(&[], &mut key).unwrap();
    key
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
