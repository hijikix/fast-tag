pub mod providers;
pub mod config;
pub mod factory;
pub mod handlers;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum StorageError {
    NotFound,
    #[allow(dead_code)]
    AccessDenied,
    NetworkError(String),
    #[allow(dead_code)]
    SerializationError(String),
    ConfigurationError(String),
    #[allow(dead_code)]
    UnknownError(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotFound => write!(f, "Resource not found"),
            StorageError::AccessDenied => write!(f, "Access denied"),
            StorageError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            StorageError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            StorageError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            StorageError::UnknownError(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl Error for StorageError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetadata {
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn upload(
        &self,
        key: &str,
        data: &[u8],
        content_type: Option<&str>,
    ) -> Result<String, StorageError>;

    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    async fn get_presigned_url(
        &self,
        key: &str,
        expires_in_secs: u64,
    ) -> Result<String, StorageError>;

    #[allow(dead_code)]
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    #[allow(dead_code)]
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;

    async fn get_metadata(&self, key: &str) -> Result<StorageMetadata, StorageError>;

    async fn list_objects(&self, prefix: Option<&str>) -> Result<Vec<String>, StorageError>;
}