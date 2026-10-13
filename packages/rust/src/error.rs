//! SDK 公开错误类型。

/// SDK 结果类型。
pub type Result<T> = std::result::Result<T, Error>;

/// Runtime 描述、传输和 Agent 响应错误。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("runtime descriptor is invalid: {0}")]
    InvalidDescriptor(String),
    #[error("runtime capability is not granted: {0}")]
    CapabilityDenied(String),
    #[error("runtime credential is invalid: {0}")]
    InvalidCredential(String),
    #[error("operation event is invalid: {0}")]
    InvalidEvent(String),
    #[error("runtime I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("runtime JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("runtime transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Agent rejected the request with HTTP {status}: {message}")]
    Agent { status: u16, message: String },
}
