use crate::storage::{StorageProvider, StorageError, StorageMetadata};
use async_trait::async_trait;
use rusoto_core::{Region, RusotoError};
use rusoto_credential::StaticProvider;
use rusoto_s3::{
    S3Client, S3, PutObjectRequest, GetObjectRequest, DeleteObjectRequest, 
    HeadObjectRequest, ListObjectsV2Request, GetObjectError, HeadObjectError,
};
use std::str::FromStr;

pub struct S3StorageProvider {
    client: S3Client,
    bucket: String,
    _region: Region,
}

impl S3StorageProvider {
    pub async fn new(
        bucket: String,
        region: String,
        access_key: String,
        secret_key: String,
        endpoint: Option<String>,
    ) -> Result<Self, StorageError> {
        let credentials_provider = StaticProvider::new(access_key, secret_key, None, None);

        let region = if let Some(endpoint_url) = endpoint {
            Region::Custom {
                name: region,
                endpoint: endpoint_url,
            }
        } else {
            Region::from_str(&region)
                .map_err(|e| StorageError::ConfigurationError(format!("Invalid region: {}", e)))?
        };

        let client = S3Client::new_with(
            rusoto_core::request::HttpClient::new()
                .map_err(|e| StorageError::ConfigurationError(e.to_string()))?,
            credentials_provider,
            region.clone(),
        );

        Ok(Self { client, bucket, _region: region })
    }
}

#[async_trait]
impl StorageProvider for S3StorageProvider {
    async fn upload(
        &self,
        key: &str,
        data: &[u8],
        content_type: Option<&str>,
    ) -> Result<String, StorageError> {
        let mut request = PutObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            body: Some(data.to_vec().into()),
            ..Default::default()
        };

        if let Some(ct) = content_type {
            request.content_type = Some(ct.to_string());
        }

        self.client
            .put_object(request)
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?;

        Ok(format!("s3://{}/{}", self.bucket, key))
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let request = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            ..Default::default()
        };

        let response = self.client
            .get_object(request)
            .await
            .map_err(|e| match e {
                RusotoError::Service(GetObjectError::NoSuchKey(_)) => StorageError::NotFound,
                _ => StorageError::NetworkError(e.to_string()),
            })?;

        let mut data = Vec::new();
        if let Some(body) = response.body {
            use tokio::io::AsyncReadExt;
            let mut reader = body.into_async_read();
            reader.read_to_end(&mut data).await
                .map_err(|e| StorageError::NetworkError(e.to_string()))?;
        }

        Ok(data)
    }

    async fn get_presigned_url(
        &self,
        key: &str,
        _expires_in_secs: u64,
    ) -> Result<String, StorageError> {
        let _request = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            ..Default::default()
        };

        // For simplicity, return a basic S3 URL
        // In production, implement proper presigned URL generation
        Ok(format!("https://{}.s3.amazonaws.com/{}", self.bucket, key))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let request = DeleteObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            ..Default::default()
        };

        self.client
            .delete_object(request)
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let request = HeadObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            ..Default::default()
        };

        match self.client.head_object(request).await {
            Ok(_) => Ok(true),
            Err(RusotoError::Service(HeadObjectError::NoSuchKey(_))) => Ok(false),
            Err(e) => Err(StorageError::NetworkError(e.to_string())),
        }
    }

    async fn get_metadata(&self, key: &str) -> Result<StorageMetadata, StorageError> {
        let request = HeadObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            ..Default::default()
        };

        let response = self.client
            .head_object(request)
            .await
            .map_err(|e| match e {
                RusotoError::Service(HeadObjectError::NoSuchKey(_)) => StorageError::NotFound,
                _ => StorageError::NetworkError(e.to_string()),
            })?;

        Ok(StorageMetadata {
            content_type: response.content_type,
            content_length: response.content_length.map(|l| l as u64),
            etag: response.e_tag,
            last_modified: response.last_modified.and_then(|lm| {
                chrono::DateTime::parse_from_rfc2822(&lm)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            }),
        })
    }

    async fn list_objects(&self, prefix: Option<&str>) -> Result<Vec<String>, StorageError> {
        let mut request = ListObjectsV2Request {
            bucket: self.bucket.clone(),
            ..Default::default()
        };

        if let Some(p) = prefix {
            request.prefix = Some(p.to_string());
        }

        let response = self.client
            .list_objects_v2(request)
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?;

        let keys = response
            .contents
            .unwrap_or_default()
            .into_iter()
            .filter_map(|obj| obj.key)
            .collect();

        Ok(keys)
    }
}