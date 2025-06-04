use crate::app::state::AppState;
use crate::auth::AuthState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiContextPass, egui};
use reqwest;
use serde::Deserialize;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
struct AuthResponse {
    poll_token: String,
    auth_url: String,
}

#[derive(Debug, Deserialize)]
struct PollResponse {
    status: String,
    jwt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
enum LoginState {
    #[default]
    Idle,
    WaitingForAuth {
        poll_token: String,
        start_time: Instant,
    },
    Success(String), // JWT token
    Error(String),
}

#[derive(Resource, Default)]
pub struct LoginResource {
    state: LoginState,
    last_poll_time: Option<Instant>,
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(LoginResource::default());
    println!("login setup");
}

pub fn update(
    mut login_resource: ResMut<LoginResource>,
    mut next_state: ResMut<NextState<AppState>>,
    mut auth_state: ResMut<AuthState>,
) {
    let now = Instant::now();
    
    match &login_resource.state {
        LoginState::WaitingForAuth { poll_token, start_time } => {
            // Timeout check (5 minutes)
            if now.duration_since(*start_time) > Duration::from_secs(300) {
                login_resource.state = LoginState::Error("Authentication timeout".to_string());
                return;
            }
            
            // Polling interval check (2 second intervals)
            if let Some(last_poll) = login_resource.last_poll_time {
                if now.duration_since(last_poll) < Duration::from_secs(2) {
                    return;
                }
            }
            
            // Execute polling to retrieve JWT
            let poll_token = poll_token.clone();
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            match rt.block_on(poll_for_jwt(&poll_token)) {
                Ok(Some(jwt)) => {
                    login_resource.state = LoginState::Success(jwt.clone());
                    auth_state.set_jwt(jwt);
                    next_state.set(AppState::Projects);
                }
                Ok(None) => {
                    // Authentication not yet completed
                    login_resource.last_poll_time = Some(now);
                }
                Err(err) => {
                    login_resource.state = LoginState::Error(err);
                }
            }
        }
        LoginState::Success(_) => {
            // Already logged in successfully, transition to Projects screen
            next_state.set(AppState::Projects);
        }
        _ => {}
    }
}

pub fn ui_system(
    mut contexts: EguiContexts,
    _current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut login_resource: ResMut<LoginResource>,
) {
    // Temporarily skip top panel and implement login UI first
    
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            
            ui.heading("Login");
            ui.add_space(50.0);
            
            match &login_resource.state {
                LoginState::Idle => {
                    // GitHub login button
                    if ui.button("🚀 Login with GitHub").clicked() {
                        start_oauth_login("github", &mut login_resource);
                    }
                    
                    ui.add_space(10.0);
                    
                    // Google login button
                    if ui.button("🔍 Login with Google").clicked() {
                        start_oauth_login("google", &mut login_resource);
                    }
                }
                LoginState::WaitingForAuth { .. } => {
                    ui.label("🔄 Waiting for authentication...");
                    ui.label("Please complete the authentication in your browser.");
                    
                    if ui.button("Cancel").clicked() {
                        login_resource.state = LoginState::Idle;
                    }
                }
                LoginState::Error(error) => {
                    ui.colored_label(egui::Color32::RED, format!("❌ Error: {}", error));
                    
                    if ui.button("Try Again").clicked() {
                        login_resource.state = LoginState::Idle;
                    }
                }
                LoginState::Success(_) => {
                    ui.label("✅ Login successful! Redirecting...");
                }
            }
            
            ui.add_space(20.0);
            
            // Development skip button
            if ui.button("Skip (Development)").clicked() {
                next_state.set(AppState::Projects);
            }
        });
    });
}

fn start_oauth_login(provider: &str, login_resource: &mut LoginResource) {
    let api_url = format!("http://localhost:8080/auth/{}", provider);
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(async {
        let response = reqwest::get(&api_url).await.map_err(|e| e.to_string())?;
        if response.status().is_success() {
            let auth_response: AuthResponse = response.json().await.map_err(|e| e.to_string())?;
            Ok(auth_response)
        } else {
            Err(format!("API response status: {}", response.status()))
        }
    }) {
        Ok(auth_response) => {
            // Open authentication URL in browser
            if let Err(e) = open::that(&auth_response.auth_url) {
                login_resource.state = LoginState::Error(format!("Failed to open browser: {}", e));
                return;
            }
            
            // Start polling
            login_resource.state = LoginState::WaitingForAuth {
                poll_token: auth_response.poll_token,
                start_time: Instant::now(),
            };
            login_resource.last_poll_time = None;
        }
        Err(e) => {
            login_resource.state = LoginState::Error(format!("Failed to call auth API: {}", e));
        }
    }
}

async fn poll_for_jwt(poll_token: &str) -> Result<Option<String>, String> {
    let poll_url = format!("http://localhost:8080/auth/poll/{}", poll_token);
    
    match reqwest::get(&poll_url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<PollResponse>().await {
                    Ok(poll_response) => {
                        match poll_response.status.as_str() {
                            "completed" => Ok(poll_response.jwt),
                            "pending" => Ok(None),
                            "expired" => Err("Authentication session expired".to_string()),
                            "failed" => Err("Authentication failed".to_string()),
                            _ => Err(format!("Unknown status: {}", poll_response.status)),
                        }
                    }
                    Err(e) => Err(format!("Failed to parse poll response: {}", e)),
                }
            } else {
                Err(format!("Poll request failed with status: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to poll for JWT: {}", e)),
    }
}

pub fn cleanup(_commands: Commands) {
    println!("login cleanup");
}

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Login), setup)
           .add_systems(Update, update.run_if(in_state(AppState::Login)))
           .add_systems(
               EguiContextPass,
               ui_system.run_if(in_state(AppState::Login)),
           )
           .add_systems(OnExit(AppState::Login), cleanup);
    }
}