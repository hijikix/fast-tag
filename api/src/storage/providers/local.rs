use crate::storage::{StorageProvider, StorageError, StorageMetadata};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct LocalStorageProvider {
    base_path: PathBuf,
}

impl LocalStorageProvider {
    pub async fn new(base_path: String) -> Result<Self, StorageError> {
        let path = PathBuf::from(base_path);
        
        if !path.exists() {
            fs::create_dir_all(&path)
                .await
                .map_err(|e| StorageError::ConfigurationError(format!("Failed to create base directory: {}", e)))?;
        }

        Ok(Self { base_path: path })
    }

    fn get_file_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

#[async_trait]
impl StorageProvider for LocalStorageProvider {
    async fn upload(
        &self,
        key: &str,
        data: &[u8],
        _content_type: Option<&str>,
    ) -> Result<String, StorageError> {
        let file_path = self.get_file_path(key);
        
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::NetworkError(format!("Failed to create directory: {}", e)))?;
        }

        let mut file = fs::File::create(&file_path)
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to create file: {}", e)))?;

        file.write_all(data)
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to write file: {}", e)))?;

        file.sync_all()
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to sync file: {}", e)))?;

        Ok(format!("file://{}", file_path.display()))
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let file_path = self.get_file_path(key);

        if !file_path.exists() {
            return Err(StorageError::NotFound);
        }

        fs::read(&file_path)
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to read file: {}", e)))
    }

    async fn get_presigned_url(
        &self,
        key: &str,
        _expires_in_secs: u64,
    ) -> Result<String, StorageError> {
        let file_path = self.get_file_path(key);
        
        if !file_path.exists() {
            return Err(StorageError::NotFound);
        }

        Ok(format!("file://{}", file_path.display()))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let file_path = self.get_file_path(key);

        if !file_path.exists() {
            return Err(StorageError::NotFound);
        }

        fs::remove_file(&file_path)
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to delete file: {}", e)))?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let file_path = self.get_file_path(key);
        Ok(file_path.exists())
    }

    async fn get_metadata(&self, key: &str) -> Result<StorageMetadata, StorageError> {
        let file_path = self.get_file_path(key);

        if !file_path.exists() {
            return Err(StorageError::NotFound);
        }

        let metadata = fs::metadata(&file_path)
            .await
            .map_err(|e| StorageError::NetworkError(format!("Failed to get metadata: {}", e)))?;

        let content_type = mime_guess::from_path(&file_path)
            .first()
            .map(|mime| mime.to_string());

        let last_modified = metadata
            .modified()
            .ok()
            .and_then(|time| chrono::DateTime::from_timestamp(
                time.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64, 0
            ));

        Ok(StorageMetadata {
            content_type,
            content_length: Some(metadata.len()),
            etag: None,
            last_modified,
        })
    }

    async fn list_objects(&self, prefix: Option<&str>) -> Result<Vec<String>, StorageError> {
        let search_path = if let Some(p) = prefix {
            self.base_path.join(p)
        } else {
            self.base_path.clone()
        };

        let mut entries = vec![];
        let mut stack = vec![search_path];

        while let Some(current_path) = stack.pop() {
            if current_path.is_file() {
                if let Ok(relative_path) = current_path.strip_prefix(&self.base_path) {
                    entries.push(relative_path.to_string_lossy().to_string());
                }
            } else if current_path.is_dir() {
                let mut dir_entries = fs::read_dir(&current_path)
                    .await
                    .map_err(|e| StorageError::NetworkError(format!("Failed to read directory: {}", e)))?;

                while let Some(entry) = dir_entries.next_entry()
                    .await
                    .map_err(|e| StorageError::NetworkError(format!("Failed to read directory entry: {}", e)))? 
                {
                    stack.push(entry.path());
                }
            }
        }

        Ok(entries)
    }
}