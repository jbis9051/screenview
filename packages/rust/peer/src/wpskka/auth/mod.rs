use ring::agreement::PublicKey;
use srp_client::SrpAuthClient;
use srp_host::SrpAuthHost;

pub mod srp_client;
pub mod srp_host;

#[derive(Debug)]
pub enum AuthScheme<const N: usize> {
    None {
        public_key: PublicKey,
        foreign_public_key: [u8; N],
    },
    SrpAuthClient(SrpAuthClient<N>),
    SrpAuthHost(SrpAuthHost<N>),
}
