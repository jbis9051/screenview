use hkdf::SimpleHkdf;
use hmac::SimpleHmac;
use srp::groups::G_2048;

pub const SVSC_VERSION: &str = "SVSC 001.000";
pub const RVD_VERSION: &str = "RVD 001.000";
pub static SRP_PARAM: &G_2048 = &G_2048;

pub type HashAlgo = blake3::Hasher;
pub type Hmac = SimpleHmac<HashAlgo>;
pub type Hkdf = SimpleHkdf<HashAlgo>;

pub const SEL_KDF_CONTEXT: &[u8; 28] = b"SEL-KeyDerivation-Unreliable";
pub const SEL_AEAD_CONTEXT: &[u8; 25] = b"SEL-Encryption-Unreliable";

pub const WPSKKA_KDF_CONTEXT: &[u8; 20] = b"WPSKKA-KeyDerivation";
pub const WPSKKA_AEAD_RELIABLE_CONTEXT: &[u8; 26] = b"WPSKKA-Encryption-Reliable";
pub const WPSKKA_AEAD_UNRELIABLE_CONTEXT: &[u8; 28] = b"WPSKKA-Encryption-Unreliable";
pub const WPSKKA_AUTH_SRP_CONTEXT: &[u8; 15] = b"WPSKKA-Auth-SRP";

pub use hmac::Mac;
