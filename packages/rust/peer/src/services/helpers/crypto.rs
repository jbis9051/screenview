use common::constants::{Hkdf, Hmac, Mac, SRP_PARAM};
use ring::{
    agreement,
    agreement::{EphemeralPrivateKey, PublicKey, UnparsedPublicKey},
    error,
    error::Unspecified,
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

pub fn parse_foreign_public(public_key: &[u8; 32]) -> UnparsedPublicKey<&[u8; 32]> {
    agreement::UnparsedPublicKey::new(&agreement::X25519, public_key)
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
    (
        (&key[0 .. 32]).try_into().unwrap(),
        (&key[32 .. 64]).try_into().unwrap(),
    )
}

pub fn kdf4(ikm: &[u8]) -> ([u8; 32], [u8; 32], [u8; 32], [u8; 32]) {
    let kdf = Hkdf::new(None, ikm);
    let mut key = [0u8; 128];
    kdf.expand(&[], &mut key).unwrap();
    (
        (&key[0 .. 32]).try_into().unwrap(),
        (&key[32 .. 64]).try_into().unwrap(),
        (&key[64 .. 96]).try_into().unwrap(),
        (&key[96 .. 128]).try_into().unwrap(),
    )
}

#[allow(clippy::type_complexity)]
pub fn diffie_hellman(
    my_private_key: EphemeralPrivateKey,
    peer_public_key: UnparsedPublicKey<&[u8; 32]>,
) -> Result<([u8; 32], [u8; 32], [u8; 32], [u8; 32]), Unspecified> {
    agreement::agree_ephemeral(
        my_private_key,
        &peer_public_key,
        ring::error::Unspecified,
        |key_material| Ok(kdf4(key_material)),
    )
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
