use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("DNS parse error: {0}")]
    DnsParse(#[from] hickory_proto::ProtoError),

    #[error("upstream request failed: {0}")]
    Upstream(#[from] reqwest::Error),

    #[error("invalid response from upstream")]
    InvalidUpstreamResponse,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, ProxyError>;
