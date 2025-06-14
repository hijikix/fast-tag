use crate::api::{ApiError, ApiResult, ApiConfig};
use uuid::Uuid;
use bevy::log::{info, warn, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub message: String,
    pub stats: ImportStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportStats {
    pub categories_created: usize,
    pub categories_updated: usize,
    pub tasks_created: usize,
    pub annotations_created: usize,
    pub errors: Vec<String>,
}

pub struct ImportApi {
    client: reqwest::Client,
    config: ApiConfig,
}

impl ImportApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            config: ApiConfig::default(),
        }
    }

    pub async fn import_coco_file(&self, token: &str, project_id: Uuid, file_path: &str) -> ApiResult<ImportResult> {
        let url = format!("{}/projects/{}/import/coco", self.config.base_url, project_id);
        info!("Starting COCO import for project {} from file: {}", project_id, file_path);
        info!("Making request to URL: {}", url);

        // Read file content
        let file_content = match std::fs::read(file_path) {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to read file {}: {}", file_path, e);
                return Err(ApiError::BadRequest(format!("Failed to read file: {}", e)));
            }
        };

        // Create multipart form
        let form = reqwest::multipart::Form::new()
            .part("file", 
                  reqwest::multipart::Part::bytes(file_content)
                      .file_name("coco_import.json")
                      .mime_str("application/json")
                      .map_err(|e| ApiError::BadRequest(format!("Failed to create multipart: {}", e)))?
            );

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await?;

        info!("Received response with status: {}", response.status());

        match response.status() {
            reqwest::StatusCode::OK => {
                let result: ImportResult = response.json().await?;
                info!("Successfully imported COCO data: {}", result.message);
                Ok(result)
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                warn!("COCO import failed: Unauthorized access for project {}", project_id);
                Err(ApiError::AuthenticationError("Unauthorized".to_string()))
            }
            reqwest::StatusCode::NOT_FOUND => {
                warn!("COCO import failed: Project {} not found or access denied", project_id);
                Err(ApiError::NotFound("Project not found or access denied".to_string()))
            }
            reqwest::StatusCode::BAD_REQUEST => {
                let error_text = response.text().await.unwrap_or_else(|_| "Bad request".to_string());
                warn!("COCO import failed: Bad request - {}", error_text);
                Err(ApiError::BadRequest(error_text))
            }
            status => {
                let error_text = response.text().await.unwrap_or_else(|_| status.to_string());
                error!("COCO import failed with status {}: {}", status, error_text);
                Err(ApiError::ServerError(format!("Server error: {}", error_text)))
            }
        }
    }
}