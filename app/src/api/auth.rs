use super::{ApiClient, ApiResult};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub poll_token: String,
    pub auth_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PollResponse {
    pub status: String,
    pub jwt: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub provider: String,
    pub provider_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoResponse {
    pub user: User,
}

pub struct AuthApi {
    client: ApiClient,
}

impl AuthApi {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }

    pub async fn start_oauth(&self, provider: &str) -> ApiResult<AuthResponse> {
        let endpoint = format!("/auth/{}", provider);
        self.client.get(&endpoint, None).await
    }

    pub async fn poll_auth(&self, poll_token: &str) -> ApiResult<PollResponse> {
        let endpoint = format!("/auth/poll/{}", poll_token);
        self.client.get(&endpoint, None).await
    }

    pub async fn get_user_info(&self, jwt: &str) -> ApiResult<User> {
        let response: UserInfoResponse = self.client.get("/me", Some(jwt)).await?;
        Ok(response.user)
    }
}

impl Default for AuthApi {
    fn default() -> Self {
        Self::new()
    }
}