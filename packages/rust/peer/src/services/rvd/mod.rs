mod client;
mod host;

pub use client::*;
pub use host::*;

pub enum RvdHandler {
    Host(RvdHostHandler),
    Cliient(RvdClientHandler),
}

#[derive(Debug, thiserror::Error)]
pub enum RvdError {
    #[error("host error: {0}")]
    Host(#[from] RvdHostError),
    #[error("client error: {0}")]
    Cliient(#[from] RvdClientError),
}
