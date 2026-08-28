use thiserror::Error;

#[derive(Error, Debug)]
pub enum PreviewError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("too large: {0} > {1}")]
    TooLarge(u64, u64),
    #[error("decode: {0}")]
    Decode(String),
    #[error("cancelled")]
    Cancelled,
    #[error("timeout")]
    Timeout,
}
