use crate::storage::{StorageProvider, StorageError, StorageMetadata};
use async_trait::async_trait;
use rusoto_core::{Region, RusotoError};
use rusoto_credential::{StaticProvider, ProvideAwsCredentials};
use rusoto_s3::{
    S3Client, S3, PutObjectRequest, GetObjectRequest, DeleteObjectRequest, 
    HeadObjectRequest, ListObjectsV2Request, GetObjectError, HeadObjectError,
    util::{PreSignedRequest, PreSignedRequestOption},
};
use std::str::FromStr;

pub struct S3StorageProvider {
    client: S3Client,
    bucket: String,
    region: Region,
    credentials: StaticProvider,
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
            credentials_provider.clone(),
            region.clone(),
        );

        Ok(Self { client, bucket, region: region.clone(), credentials: credentials_provider })
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
        expires_in_secs: u64,
    ) -> Result<String, StorageError> {
        // Check if file exists first
        if !self.exists(key).await? {
            return Err(StorageError::NotFound);
        }

        let request = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            ..Default::default()
        };

        // Generate presigned URL using rusoto_s3
        let credentials = match self.credentials.credentials().await {
            Ok(creds) => creds,
            Err(e) => {
                eprintln!("Failed to get credentials: {}", e);
                return Err(StorageError::NetworkError(format!("Failed to get credentials: {}", e)));
            }
        };

        let options = PreSignedRequestOption {
            expires_in: std::time::Duration::from_secs(expires_in_secs),
        };

        // Generate presigned URL with the correct region/endpoint
        let mut presigned_url = request.get_presigned_url(&self.region, &credentials, &options);
        
        eprintln!("Generated presigned URL: {}", presigned_url);
        eprintln!("Region: {:?}", self.region);
        
        // For custom endpoints (like MinIO), we need to replace the host in the URL
        if let Region::Custom { endpoint, .. } = &self.region {
            // Parse the generated URL and replace the host with our custom endpoint
            if let Ok(mut url) = url::Url::parse(&presigned_url) {
                if let Ok(custom_url) = url::Url::parse(endpoint) {
                    if let Err(e) = url.set_host(custom_url.host_str()) {
                        eprintln!("Failed to set host: {:?}", e);
                        return Err(StorageError::ConfigurationError(format!("Failed to set host: {:?}", e)));
                    }
                    if let Some(port) = custom_url.port() {
                        if let Err(e) = url.set_port(Some(port)) {
                            eprintln!("Failed to set port: {:?}", e);
                            return Err(StorageError::ConfigurationError(format!("Failed to set port: {:?}", e)));
                        }
                    }
                    if let Err(e) = url.set_scheme(custom_url.scheme()) {
                        eprintln!("Failed to set scheme: {:?}", e);
                        return Err(StorageError::ConfigurationError(format!("Failed to set scheme: {:?}", e)));
                    }
                    presigned_url = url.to_string();
                }
            }
        }
        
        Ok(presigned_url)
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