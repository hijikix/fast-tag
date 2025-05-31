use crate::app::state::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use reqwest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AuthUrlResponse {
    auth_url: String,
}

pub fn setup(_commands: Commands) {
    println!("login setup");
}

pub fn update() {
    // ログインページの更新ロジック（現在は空）
}

pub fn ui_system(
    mut contexts: EguiContexts,
    _current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // トップパネルは一時的にスキップして、まずはログインUIを実装
    
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            
            ui.heading("Login");
            ui.add_space(50.0);
            
            // GitHub login button
            if ui.button("🚀 Login with GitHub").clicked() {
                handle_oauth_login("github");
            }
            
            ui.add_space(10.0);
            
            // Google login button
            if ui.button("🔍 Login with Google").clicked() {
                handle_oauth_login("google");
            }
            
            ui.add_space(20.0);
            
            // Development skip button
            if ui.button("Skip (Development)").clicked() {
                next_state.set(AppState::List);
            }
        });
    });
}

fn handle_oauth_login(provider: &str) {
    let api_url = format!("http://localhost:8080/auth/{}", provider);
    
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match reqwest::get(&api_url).await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<AuthUrlResponse>().await {
                            Ok(auth_response) => {
                                println!("Opening auth URL: {}", auth_response.auth_url);
                                if let Err(e) = open::that(&auth_response.auth_url) {
                                    println!("Failed to open browser: {}", e);
                                }
                            }
                            Err(e) => {
                                println!("Failed to parse JSON response: {}", e);
                            }
                        }
                    } else {
                        println!("API response status: {}", response.status());
                    }
                }
                Err(e) => {
                    println!("Failed to call auth API: {}", e);
                }
            }
        });
    });
}

pub fn cleanup(_commands: Commands) {
    println!("login cleanup");
}