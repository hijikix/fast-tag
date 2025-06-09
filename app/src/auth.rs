use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Default)]
pub struct AuthState {
    pub jwt: Option<String>,
}

impl AuthState {
    pub fn is_authenticated(&self) -> bool {
        self.jwt.is_some()
    }
    
    pub fn set_jwt(&mut self, jwt: String) {
        self.jwt = Some(jwt);
    }
    
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.jwt = None;
    }
    
    pub fn get_jwt(&self) -> Option<&String> {
        self.jwt.as_ref()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct User {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub email: String,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub avatar_url: Option<String>,
    #[allow(dead_code)]
    pub provider: String,
    #[allow(dead_code)]
    pub provider_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoResponse {
    #[allow(dead_code)]
    pub user: User,
}

#[derive(Resource, Default)]
pub struct UserState {
    pub user: Option<User>,
    pub fetch_error: Option<String>,
    pub is_fetching: bool,
}

impl UserState {
    #[allow(dead_code)]
    pub fn set_user(&mut self, user: User) {
        self.user = Some(user);
        self.fetch_error = None;
    }
    
    #[allow(dead_code)]
    pub fn set_error(&mut self, error: String) {
        self.fetch_error = Some(error);
        self.is_fetching = false;
    }
    
    #[allow(dead_code)]
    pub fn start_fetching(&mut self) {
        self.is_fetching = true;
        self.fetch_error = None;
    }
    
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.user = None;
        self.fetch_error = None;
        self.is_fetching = false;
    }
}

#[allow(dead_code)]
pub async fn fetch_user_info(jwt: &str) -> Result<User, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/me")
        .bearer_auth(jwt)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let user_info: UserInfoResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(user_info.user)
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Project {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[allow(dead_code)]
    pub owner_id: String,
    pub storage_config: Option<serde_json::Value>,
    pub created_at: String,
    #[allow(dead_code)]
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

#[derive(Resource, Default)]
pub struct ProjectsState {
    pub projects: Vec<Project>,
    pub fetch_error: Option<String>,
    pub is_fetching: bool,
}

impl ProjectsState {
    pub fn set_projects(&mut self, projects: Vec<Project>) {
        self.projects = projects;
        self.fetch_error = None;
        self.is_fetching = false;
    }
    
    pub fn set_error(&mut self, error: String) {
        self.fetch_error = Some(error);
        self.is_fetching = false;
    }
    
    pub fn start_fetching(&mut self) {
        self.is_fetching = true;
        self.fetch_error = None;
    }
    
    pub fn add_project(&mut self, project: Project) {
        self.projects.push(project);
    }
    
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.projects.clear();
        self.fetch_error = None;
        self.is_fetching = false;
    }
}

pub async fn fetch_projects(jwt: &str) -> Result<Vec<Project>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/projects")
        .bearer_auth(jwt)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let projects_response: ProjectsListResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(projects_response.projects)
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

pub async fn create_project(jwt: &str, name: &str, description: Option<&str>) -> Result<Project, String> {
    let client = reqwest::Client::new();
    let request_body = CreateProjectRequest {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
    };
    
    let response = client
        .post("http://localhost:8080/projects")
        .bearer_auth(jwt)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let project_response: ProjectResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(project_response.project)
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

pub async fn update_project(jwt: &str, project_id: &str, name: &str, description: Option<&str>) -> Result<Project, String> {
    let client = reqwest::Client::new();
    let request_body = UpdateProjectRequest {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
    };
    
    let response = client
        .put(format!("http://localhost:8080/projects/{}", project_id))
        .bearer_auth(jwt)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let project_response: ProjectResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(project_response.project)
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("API error: {}", error_text))
    }
}

pub async fn delete_project(jwt: &str, project_id: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    
    let response = client
        .delete(format!("http://localhost:8080/projects/{}", project_id))
        .bearer_auth(jwt)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("API error: {}", error_text))
    }
}

pub async fn update_project_storage_config(jwt: &str, project_id: &str, storage_config: serde_json::Value) -> Result<Project, String> {
    let client = reqwest::Client::new();
    let request_body = UpdateStorageConfigRequest {
        storage_config,
    };
    
    let response = client
        .put(format!("http://localhost:8080/projects/{}/storage-config", project_id))
        .bearer_auth(jwt)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let project_response: ProjectResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(project_response.project)
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("API error: {}", error_text))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Task {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub project_id: String,
    pub name: String,
    pub resource_url: Option<String>,
    pub status: String,
    pub created_at: String,
    #[allow(dead_code)]
    pub updated_at: String,
    #[allow(dead_code)]
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
pub struct TaskResponse {
    #[allow(dead_code)]
    pub task: Task,
    #[allow(dead_code)]
    pub resolved_resource_url: Option<String>,
}


pub async fn fetch_tasks(jwt: &str, project_id: &str) -> Result<Vec<TaskWithResolvedUrl>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://localhost:8080/projects/{}/tasks", project_id))
        .bearer_auth(jwt)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let tasks_response: TasksListResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(tasks_response.tasks)
    } else {
        Err(format!("API error: {}", response.status()))
    }
}

#[allow(dead_code)]
pub async fn create_task(jwt: &str, project_id: &str, name: &str, resource_url: Option<&str>) -> Result<Task, String> {
    let client = reqwest::Client::new();
    let request_body = CreateTaskRequest {
        name: name.to_string(),
        resource_url: resource_url.map(|s| s.to_string()),
    };
    
    let response = client
        .post(format!("http://localhost:8080/projects/{}/tasks", project_id))
        .bearer_auth(jwt)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let task_response: TaskResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(task_response.task)
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("API error: {}", error_text))
    }
}