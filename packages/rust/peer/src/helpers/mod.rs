mod anti_replay;
pub mod cipher_reliable_peer;
pub mod cipher_unreliable_peer;
pub mod crypto;
pub(crate) mod left_pad;

// Differs from the spec currently, but we give ourselves a larger margin because of
// multithreading consistency concerns
pub const MAX_NONCE: u64 = i64::MAX as u64;
