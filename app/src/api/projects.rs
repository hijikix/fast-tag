use super::{ApiClient, ApiResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub storage_config: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectsListResponse {
    pub projects: Vec<Project>,
}

#[derive(Debug, Serialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateStorageConfigRequest {
    pub storage_config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ProjectResponse {
    pub project: Project,
}

pub struct ProjectsApi {
    client: ApiClient,
}

impl ProjectsApi {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }

    pub async fn list_projects(&self, jwt: &str) -> ApiResult<Vec<Project>> {
        let response: ProjectsListResponse = self.client.get("/projects", Some(jwt)).await?;
        Ok(response.projects)
    }

    pub async fn create_project(
        &self,
        jwt: &str,
        name: &str,
        description: Option<&str>,
    ) -> ApiResult<Project> {
        let request = CreateProjectRequest {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
        };
        let response: ProjectResponse = self.client.post("/projects", &request, Some(jwt)).await?;
        Ok(response.project)
    }

    pub async fn update_project(
        &self,
        jwt: &str,
        project_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> ApiResult<Project> {
        let request = UpdateProjectRequest {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
        };
        let endpoint = format!("/projects/{}", project_id);
        let response: ProjectResponse = self.client.put(&endpoint, &request, Some(jwt)).await?;
        Ok(response.project)
    }

    pub async fn delete_project(&self, jwt: &str, project_id: &str) -> ApiResult<()> {
        let endpoint = format!("/projects/{}", project_id);
        self.client.delete(&endpoint, Some(jwt)).await
    }

    pub async fn update_storage_config(
        &self,
        jwt: &str,
        project_id: &str,
        storage_config: serde_json::Value,
    ) -> ApiResult<Project> {
        let request = UpdateStorageConfigRequest { storage_config };
        let endpoint = format!("/projects/{}/storage-config", project_id);
        let response: ProjectResponse = self.client.put(&endpoint, &request, Some(jwt)).await?;
        Ok(response.project)
    }
}

impl Default for ProjectsApi {
    fn default() -> Self {
        Self::new()
    }
}