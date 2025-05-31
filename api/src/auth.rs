use actix_web::{web, HttpResponse, Responder};
use oauth2::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    AuthUrl, TokenUrl, basic::BasicClient, TokenResponse
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, Header, EncodingKey, DecodingKey};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub provider: String,
    pub provider_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUserInfo {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubEmail {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct AuthCallback {
    pub code: String,
    #[allow(dead_code)]
    pub state: String,
}

#[derive(Clone)]
pub struct OAuthConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_url: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub github_redirect_url: String,
    pub jwt_secret: String,
}

impl OAuthConfig {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        Ok(Self {
            google_client_id: std::env::var("GOOGLE_CLIENT_ID")?,
            google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET")?,
            google_redirect_url: std::env::var("GOOGLE_REDIRECT_URL")?,
            github_client_id: std::env::var("GITHUB_CLIENT_ID")?,
            github_client_secret: std::env::var("GITHUB_CLIENT_SECRET")?,
            github_redirect_url: std::env::var("GITHUB_REDIRECT_URL")?,
            jwt_secret: std::env::var("JWT_SECRET")?,
        })
    }
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    #[allow(dead_code)]
    decoding_key: DecodingKey,
}

impl JwtManager {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
        }
    }

    pub fn generate_token(&self, user_id: &str, email: &str, name: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = Utc::now();
        let exp = now + chrono::Duration::hours(24);

        let claims = Claims {
            sub: user_id.to_owned(),
            email: email.to_owned(),
            name: name.to_owned(),
            iat: now.timestamp() as usize,
            exp: exp.timestamp() as usize,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
    }
}

pub async fn google_login(config: web::Data<OAuthConfig>) -> impl Responder {
    let client = BasicClient::new(
        ClientId::new(config.google_client_id.clone()),
        Some(ClientSecret::new(config.google_client_secret.clone())),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
        Some(TokenUrl::new("https://www.googleapis.com/oauth2/v4/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(config.google_redirect_url.clone()).unwrap());

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish()
}

pub async fn github_login(config: web::Data<OAuthConfig>) -> impl Responder {
    let client = BasicClient::new(
        ClientId::new(config.github_client_id.clone()),
        Some(ClientSecret::new(config.github_client_secret.clone())),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(config.github_redirect_url.clone()).unwrap());

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish()
}

pub async fn google_callback(
    query: web::Query<AuthCallback>,
    pool: web::Data<Pool<Postgres>>,
    config: web::Data<OAuthConfig>,
) -> impl Responder {
    let client = BasicClient::new(
        ClientId::new(config.google_client_id.clone()),
        Some(ClientSecret::new(config.google_client_secret.clone())),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
        Some(TokenUrl::new("https://www.googleapis.com/oauth2/v4/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(config.google_redirect_url.clone()).unwrap());

    let token = match client.exchange_code(AuthorizationCode::new(query.code.clone())).request_async(oauth2::reqwest::async_http_client).await {
        Ok(token) => token,
        Err(_) => return HttpResponse::BadRequest().json("Failed to exchange code for token"),
    };

    let user_info: GoogleUserInfo = match reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
    {
        Ok(response) => match response.json().await {
            Ok(info) => info,
            Err(_) => return HttpResponse::BadRequest().json("Failed to get user info"),
        },
        Err(_) => return HttpResponse::BadRequest().json("Failed to request user info"),
    };

    match create_or_get_user(&pool, &user_info.email, &user_info.name, user_info.picture.as_deref(), "google", &user_info.id).await {
        Ok(user) => {
            let jwt_manager = JwtManager::new(&config.jwt_secret);
            
            match jwt_manager.generate_token(&user.id.to_string(), &user.email, &user.name) {
                Ok(token) => HttpResponse::Ok().json(AuthResponse { token, user }),
                Err(_) => HttpResponse::InternalServerError().json("Failed to generate token"),
            }
        }
        Err(_) => HttpResponse::InternalServerError().json("Failed to create user"),
    }
}

pub async fn github_callback(
    query: web::Query<AuthCallback>,
    pool: web::Data<Pool<Postgres>>,
    config: web::Data<OAuthConfig>,
) -> impl Responder {
    let client = BasicClient::new(
        ClientId::new(config.github_client_id.clone()),
        Some(ClientSecret::new(config.github_client_secret.clone())),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(config.github_redirect_url.clone()).unwrap());

    let token = match client.exchange_code(AuthorizationCode::new(query.code.clone())).request_async(oauth2::reqwest::async_http_client).await {
        Ok(token) => token,
        Err(_) => return HttpResponse::BadRequest().json("Failed to exchange code for token"),
    };

    let user_info: GitHubUserInfo = match reqwest::Client::new()
        .get("https://api.github.com/user")
        .bearer_auth(token.access_token().secret())
        .header("User-Agent", "fast-tag-app")
        .send()
        .await
    {
        Ok(response) => match response.json().await {
            Ok(info) => info,
            Err(_) => return HttpResponse::BadRequest().json("Failed to get user info"),
        },
        Err(_) => return HttpResponse::BadRequest().json("Failed to request user info"),
    };

    let email = match user_info.email {
        Some(email) => email,
        None => {
            let emails: Vec<GitHubEmail> = match reqwest::Client::new()
                .get("https://api.github.com/user/emails")
                .bearer_auth(token.access_token().secret())
                .header("User-Agent", "fast-tag-app")
                .send()
                .await
            {
                Ok(response) => match response.json().await {
                    Ok(emails) => emails,
                    Err(_) => return HttpResponse::BadRequest().json("Failed to get user emails"),
                },
                Err(_) => return HttpResponse::BadRequest().json("Failed to request user emails"),
            };

            match emails.iter().find(|e| e.primary && e.verified) {
                Some(email) => email.email.clone(),
                None => return HttpResponse::BadRequest().json("No verified primary email found"),
            }
        }
    };

    let name = user_info.name.unwrap_or(user_info.login.clone());

    match create_or_get_user(&pool, &email, &name, user_info.avatar_url.as_deref(), "github", &user_info.id.to_string()).await {
        Ok(user) => {
            let jwt_manager = JwtManager::new(&config.jwt_secret);
            
            match jwt_manager.generate_token(&user.id.to_string(), &user.email, &user.name) {
                Ok(token) => HttpResponse::Ok().json(AuthResponse { token, user }),
                Err(_) => HttpResponse::InternalServerError().json("Failed to generate token"),
            }
        }
        Err(_) => HttpResponse::InternalServerError().json("Failed to create user"),
    }
}

async fn create_or_get_user(
    pool: &Pool<Postgres>,
    email: &str,
    name: &str,
    avatar_url: Option<&str>,
    provider: &str,
    provider_id: &str,
) -> Result<User, sqlx::Error> {
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT id, email, name, avatar_url, provider, provider_id, created_at, updated_at FROM users WHERE email = $1 AND provider = $2"
    )
    .bind(email)
    .bind(provider)
    .fetch_optional(pool)
    .await?;

    match existing_user {
        Some(user) => Ok(user),
        None => {
            let user_id = Uuid::new_v4();
            let now = Utc::now();

            sqlx::query(
                "INSERT INTO users (id, email, name, avatar_url, provider, provider_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
            )
            .bind(&user_id)
            .bind(email)
            .bind(name)
            .bind(avatar_url)
            .bind(provider)
            .bind(provider_id)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;

            Ok(User {
                id: user_id,
                email: email.to_string(),
                name: name.to_string(),
                avatar_url: avatar_url.map(String::from),
                provider: provider.to_string(),
                provider_id: provider_id.to_string(),
                created_at: now,
                updated_at: now,
            })
        }
    }
}