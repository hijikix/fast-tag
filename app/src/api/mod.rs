pub mod client;
pub mod auth;
pub mod projects;
pub mod tasks;
pub mod annotations;
pub mod categories;
pub mod sync;
pub mod resources;

use std::fmt;

#[derive(Debug, Clone)]
pub enum ApiError {
    NetworkError(String),
    ParseError(String),
    AuthenticationError(String),
    BadRequest(String),
    ServerError(String),
    NotFound(String),
    Unknown(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ApiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ApiError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::ServerError(msg) => write!(f, "Server error: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        ApiError::NetworkError(error.to_string())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub base_url: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        }
    }
}


// Re-export common types and functions
pub use client::ApiClient;
// Note: Individual API modules are re-exported as needed