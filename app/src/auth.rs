use bevy::prelude::*;
use crate::api::{auth::AuthApi, projects::ProjectsApi, tasks::TasksApi};

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

// Re-export types from API modules
pub use crate::api::auth::User;

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
    let auth_api = AuthApi::new();
    auth_api.get_user_info(jwt).await.map_err(|e| e.to_string())
}

// Re-export types from API modules
pub use crate::api::projects::Project;

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
    let projects_api = ProjectsApi::new();
    projects_api.list_projects(jwt).await.map_err(|e| e.to_string())
}

pub async fn create_project(jwt: &str, name: &str, description: Option<&str>) -> Result<Project, String> {
    let projects_api = ProjectsApi::new();
    projects_api.create_project(jwt, name, description).await.map_err(|e| e.to_string())
}

pub async fn update_project(jwt: &str, project_id: &str, name: &str, description: Option<&str>) -> Result<Project, String> {
    let projects_api = ProjectsApi::new();
    projects_api.update_project(jwt, project_id, name, description).await.map_err(|e| e.to_string())
}

pub async fn delete_project(jwt: &str, project_id: &str) -> Result<(), String> {
    let projects_api = ProjectsApi::new();
    projects_api.delete_project(jwt, project_id).await.map_err(|e| e.to_string())
}

pub async fn update_project_storage_config(jwt: &str, project_id: &str, storage_config: serde_json::Value) -> Result<Project, String> {
    let projects_api = ProjectsApi::new();
    projects_api.update_storage_config(jwt, project_id, storage_config).await.map_err(|e| e.to_string())
}

// Re-export types from API modules
pub use crate::api::tasks::{Task, TaskWithResolvedUrl};



#[allow(dead_code)]
pub async fn create_task(jwt: &str, project_id: &str, name: &str, resource_url: Option<&str>) -> Result<Task, String> {
    let tasks_api = TasksApi::new();
    tasks_api.create_task(jwt, project_id, name, resource_url).await.map_err(|e| e.to_string())
}