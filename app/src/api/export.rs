use crate::api::{ApiError, ApiResult, ApiConfig};
use uuid::Uuid;
use bevy::log::{info, warn, error};

pub struct ExportApi {
    client: reqwest::Client,
    config: ApiConfig,
}

impl ExportApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            config: ApiConfig::default(),
        }
    }

    pub async fn download_coco_export(&self, token: &str, project_id: Uuid) -> ApiResult<Vec<u8>> {
        let url = format!("{}/projects/{}/export/coco", self.config.base_url, project_id);
        info!("Starting COCO export download for project {}", project_id);
        info!("Making request to URL: {}", url);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        info!("Received response with status: {}", response.status());

        match response.status() {
            reqwest::StatusCode::OK => {
                let bytes = response.bytes().await?;
                info!("Successfully downloaded COCO export data: {} bytes", bytes.len());
                Ok(bytes.to_vec())
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                warn!("COCO export failed: Unauthorized access for project {}", project_id);
                Err(ApiError::AuthenticationError("Unauthorized".to_string()))
            }
            reqwest::StatusCode::NOT_FOUND => {
                warn!("COCO export failed: Project {} not found or access denied", project_id);
                Err(ApiError::NotFound("Project not found or access denied".to_string()))
            }
            status => {
                let error_text = response.text().await.unwrap_or_else(|_| status.to_string());
                error!("COCO export failed with status {}: {}", status, error_text);
                Err(ApiError::ServerError(format!("Server error: {}", error_text)))
            }
        }
    }

}