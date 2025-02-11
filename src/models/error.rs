use serde_json;
use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Server response error: {status_code} - {message}")]
    ServerResponse {
        status_code: StatusCode,
        message: String,
    },

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Operation timed out after {0} seconds")]
    Timeout(u64),
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ParseError(pub String);
