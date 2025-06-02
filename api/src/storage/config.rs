use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StorageConfig {
    #[serde(rename = "s3")]
    S3 {
        bucket: String,
        region: String,
        access_key: String,
        secret_key: String,
        endpoint: Option<String>, // For MinIO compatibility
    },
    #[serde(rename = "azure")]
    Azure {
        account_name: String,
        account_key: String,
        container_name: String,
    },
    #[serde(rename = "gcs")]
    GoogleCloudStorage {
        bucket: String,
        project_id: String,
        service_account_key: String, // JSON key as string
    },
    #[serde(rename = "local")]
    Local {
        base_path: String,
    },
}

impl StorageConfig {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            StorageConfig::S3 { bucket, access_key, secret_key, .. } => {
                if bucket.is_empty() || access_key.is_empty() || secret_key.is_empty() {
                    return Err("S3 configuration requires bucket, access_key, and secret_key".to_string());
                }
            }
            StorageConfig::Azure { account_name, account_key, container_name } => {
                if account_name.is_empty() || account_key.is_empty() || container_name.is_empty() {
                    return Err("Azure configuration requires account_name, account_key, and container_name".to_string());
                }
            }
            StorageConfig::GoogleCloudStorage { bucket, project_id, service_account_key } => {
                if bucket.is_empty() || project_id.is_empty() || service_account_key.is_empty() {
                    return Err("GCS configuration requires bucket, project_id, and service_account_key".to_string());
                }
            }
            StorageConfig::Local { base_path } => {
                if base_path.is_empty() {
                    return Err("Local configuration requires base_path".to_string());
                }
            }
        }
        Ok(())
    }
}