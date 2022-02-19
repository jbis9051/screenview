use hkdf::SimpleHkdf;
use hmac::SimpleHmac;
use srp::groups::G_2048;

pub const SVSC_VERSION: &str = "SVSC 001.000";
pub const RVD_VERSION: &str = "RVD 001.000";
pub static SRP_PARAM: &G_2048 = &G_2048;

pub type HashAlgo = blake3::Hasher;
pub type Hmac = SimpleHmac<HashAlgo>;
pub type Hkdf = SimpleHkdf<HashAlgo>;

pub use hmac::Mac;
