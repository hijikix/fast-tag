use super::{ApiClient, ApiResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub resource_url: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TaskWithResolvedUrl {
    #[serde(flatten)]
    pub task: Task,
    pub resolved_resource_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TasksListResponse {
    pub tasks: Vec<TaskWithResolvedUrl>,
}

#[derive(Debug, Serialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub resource_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateTaskRequest {
    pub name: String,
    pub resource_url: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TaskResponse {
    pub task: Task,
    pub resolved_resource_url: Option<String>,
}

pub struct TasksApi {
    client: ApiClient,
}

impl TasksApi {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }

    pub async fn list_tasks(&self, jwt: &str, project_id: &str) -> ApiResult<Vec<TaskWithResolvedUrl>> {
        let endpoint = format!("/projects/{}/tasks", project_id);
        let response: TasksListResponse = self.client.get(&endpoint, Some(jwt)).await?;
        Ok(response.tasks)
    }

    pub async fn create_task(
        &self,
        jwt: &str,
        project_id: &str,
        name: &str,
        resource_url: Option<&str>,
    ) -> ApiResult<Task> {
        let request = CreateTaskRequest {
            name: name.to_string(),
            resource_url: resource_url.map(|s| s.to_string()),
        };
        let endpoint = format!("/projects/{}/tasks", project_id);
        let response: TaskResponse = self.client.post(&endpoint, &request, Some(jwt)).await?;
        Ok(response.task)
    }

    #[allow(dead_code)]
    pub async fn update_task(
        &self,
        jwt: &str,
        project_id: &str,
        task_id: &str,
        name: &str,
        resource_url: Option<&str>,
        status: &str,
    ) -> ApiResult<Task> {
        let request = UpdateTaskRequest {
            name: name.to_string(),
            resource_url: resource_url.map(|s| s.to_string()),
            status: status.to_string(),
        };
        let endpoint = format!("/projects/{}/tasks/{}", project_id, task_id);
        let response: TaskResponse = self.client.put(&endpoint, &request, Some(jwt)).await?;
        Ok(response.task)
    }

    #[allow(dead_code)]
    pub async fn delete_task(&self, jwt: &str, project_id: &str, task_id: &str) -> ApiResult<()> {
        let endpoint = format!("/projects/{}/tasks/{}", project_id, task_id);
        self.client.delete(&endpoint, Some(jwt)).await
    }
}

impl Default for TasksApi {
    fn default() -> Self {
        Self::new()
    }
}