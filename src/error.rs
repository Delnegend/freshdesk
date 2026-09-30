use std::path::PathBuf;
use thiserror::Error;

pub type Result<T, E = FreshdeskError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum FreshdeskError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("API error (status {status}): {message} (code: {code:?})")]
    Api {
        status: u16,
        code: Option<String>,
        message: String,
        errors: Option<Vec<serde_json::Value>>,
    },

    #[error("Rate limited: retry after {retry_after_seconds:?} seconds")]
    RateLimited { retry_after_seconds: Option<u64> },

    #[error("Authentication required: {0}")]
    Authentication(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Session file error at {path}: {message}")]
    SessionFile { path: PathBuf, message: String },

    #[error("Login helper error: {0}")]
    LoginHelper(String),
}
