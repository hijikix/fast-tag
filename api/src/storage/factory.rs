use crate::storage::{StorageProvider, StorageError};
use crate::storage::config::StorageConfig;
use crate::storage::providers::{S3StorageProvider, AzureStorageProvider, GcsStorageProvider, LocalStorageProvider};
use std::sync::Arc;

pub async fn create_storage_provider(config: &StorageConfig) -> Result<Arc<dyn StorageProvider>, StorageError> {
    match config {
        StorageConfig::S3 { bucket, region, access_key, secret_key, endpoint } => {
            let provider = S3StorageProvider::new(
                bucket.clone(),
                region.clone(),
                access_key.clone(),
                secret_key.clone(),
                endpoint.clone(),
            ).await?;
            Ok(Arc::new(provider))
        }
        StorageConfig::Azure { account_name, account_key, container_name } => {
            let provider = AzureStorageProvider::new(
                account_name.clone(),
                account_key.clone(),
                container_name.clone(),
            )?;
            Ok(Arc::new(provider))
        }
        StorageConfig::GoogleCloudStorage { bucket, project_id, service_account_key } => {
            let provider = GcsStorageProvider::new(
                bucket.clone(),
                project_id.clone(),
                service_account_key.clone(),
            ).await?;
            Ok(Arc::new(provider))
        }
        StorageConfig::Local { base_path } => {
            let provider = LocalStorageProvider::new(base_path.clone()).await?;
            Ok(Arc::new(provider))
        }
    }
}

pub async fn create_storage_provider_from_project(
    project: &crate::projects::Project,
) -> Result<Arc<dyn StorageProvider>, StorageError> {
    let storage_config = project.storage_config
        .as_ref()
        .ok_or_else(|| StorageError::ConfigurationError("No storage configuration found for project".to_string()))?;

    let config: StorageConfig = serde_json::from_value(storage_config.clone())
        .map_err(|e| StorageError::ConfigurationError(format!("Invalid storage configuration: {}", e)))?;

    create_storage_provider(&config).await
}