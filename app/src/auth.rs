use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;

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

fn get_api_base_url() -> String {
    env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

#[allow(dead_code)]
pub async fn fetch_user_info(jwt: &str) -> Result<User, String> {
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/me", base_url))
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
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/projects", base_url))
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
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    let request_body = CreateProjectRequest {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
    };
    
    let response = client
        .post(format!("{}/projects", base_url))
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
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    let request_body = UpdateProjectRequest {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
    };
    
    let response = client
        .put(format!("{}/projects/{}", base_url, project_id))
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

pub async fn fetch_storage_url(jwt: &str, project_id: &str, storage_key: &str) -> Result<String, String> {
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    
    // URL encode the storage key to handle special characters like Japanese text
    let encoded_storage_key = urlencoding::encode(storage_key);
    
    let response = client
        .get(format!("{}/projects/{}/storage/{}/url", base_url, project_id, encoded_storage_key))
        .bearer_auth(jwt)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let url_response: StorageUrlResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(url_response.download_url)
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("API error: {}", error_text))
    }
}

pub async fn delete_project(jwt: &str, project_id: &str) -> Result<(), String> {
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    
    let response = client
        .delete(format!("{}/projects/{}", base_url, project_id))
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

#[derive(Debug, Deserialize)]
pub struct TasksListResponse {
    pub tasks: Vec<Task>,
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
}

#[derive(Debug, Deserialize)]
pub struct StorageUrlResponse {
    pub download_url: String,
}


pub async fn fetch_tasks(jwt: &str, project_id: &str) -> Result<Vec<Task>, String> {
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/projects/{}/tasks", base_url, project_id))
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

pub async fn fetch_task_by_id(jwt: &str, project_id: &str, task_id: &str) -> Result<Task, String> {
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/projects/{}/tasks/{}", base_url, project_id, task_id))
        .bearer_auth(jwt)
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

#[allow(dead_code)]
pub async fn create_task(jwt: &str, project_id: &str, name: &str, resource_url: Option<&str>) -> Result<Task, String> {
    let base_url = get_api_base_url();
    let client = reqwest::Client::new();
    let request_body = CreateTaskRequest {
        name: name.to_string(),
        resource_url: resource_url.map(|s| s.to_string()),
    };
    
    let response = client
        .post(format!("{}/projects/{}/tasks", base_url, project_id))
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