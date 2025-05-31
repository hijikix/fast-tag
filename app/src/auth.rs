use bevy::prelude::*;
use serde::Deserialize;

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
    pub user: User,
}

#[derive(Resource, Default)]
pub struct UserState {
    pub user: Option<User>,
    pub fetch_error: Option<String>,
    pub is_fetching: bool,
}

impl UserState {
    pub fn set_user(&mut self, user: User) {
        self.user = Some(user);
        self.fetch_error = None;
    }
    
    pub fn set_error(&mut self, error: String) {
        self.fetch_error = Some(error);
        self.is_fetching = false;
    }
    
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