use srp_client::SrpAuthClient;
use srp_host::SrpAuthHost;

pub mod srp_client;
pub mod srp_host;

pub enum AuthScheme {
    SrpAuthClient(SrpAuthClient),
    SrpAuthHost(SrpAuthHost),
}
