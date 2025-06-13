use super::{ApiClient, ApiResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Clone)]
pub struct SyncRequest {
    pub prefix: Option<String>,
    pub file_extensions: Option<Vec<String>>,
    pub overwrite_existing: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SyncResponse {
    pub sync_id: Uuid,
    pub total_files: usize,
    pub tasks_created: usize,
    pub tasks_skipped: usize,
    pub errors: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct SyncProgress {
    pub total_files: usize,
    pub processed_files: usize,
    pub tasks_created: usize,
    pub tasks_skipped: usize,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct SyncStatus {
    pub sync_id: Uuid,
    pub status: String, // "pending", "running", "completed", "failed"
    pub progress: Option<SyncProgress>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SyncStatusResponse {
    pub sync_status: SyncStatus,
}

pub struct SyncApi {
    client: ApiClient,
}

impl SyncApi {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }

    pub async fn start_sync(
        &self,
        jwt: &str,
        project_id: Uuid,
        request: &SyncRequest,
    ) -> ApiResult<SyncResponse> {
        let endpoint = format!("/projects/{}/sync", project_id);
        self.client.post(&endpoint, request, Some(jwt)).await
    }

    #[allow(dead_code)]
    pub async fn get_sync_status(
        &self,
        jwt: &str,
        project_id: Uuid,
        sync_id: Uuid,
    ) -> ApiResult<SyncStatus> {
        let endpoint = format!("/projects/{}/sync/{}", project_id, sync_id);
        let response: SyncStatusResponse = self.client.get(&endpoint, Some(jwt)).await?;
        Ok(response.sync_status)
    }

    #[allow(dead_code)]
    pub async fn cancel_sync(
        &self,
        jwt: &str,
        project_id: Uuid,
        sync_id: Uuid,
    ) -> ApiResult<()> {
        let endpoint = format!("/projects/{}/sync/{}/cancel", project_id, sync_id);
        self.client.post(&endpoint, &(), Some(jwt)).await
    }

    #[allow(dead_code)]
    pub async fn list_sync_history(
        &self,
        jwt: &str,
        project_id: Uuid,
    ) -> ApiResult<Vec<SyncStatus>> {
        let endpoint = format!("/projects/{}/sync/history", project_id);
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct SyncHistoryResponse {
            sync_history: Vec<SyncStatus>,
        }
        let response: SyncHistoryResponse = self.client.get(&endpoint, Some(jwt)).await?;
        Ok(response.sync_history)
    }
}

impl Default for SyncApi {
    fn default() -> Self {
        Self::new()
    }
}