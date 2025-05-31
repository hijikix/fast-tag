use crate::app::state::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

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
                // TODO: Implement GitHub OAuth
                next_state.set(AppState::List);
            }
            
            ui.add_space(10.0);
            
            // Google login button
            if ui.button("🔍 Login with Google").clicked() {
                // TODO: Implement Google OAuth
                next_state.set(AppState::List);
            }
            
            ui.add_space(20.0);
            
            // Development skip button
            if ui.button("Skip (Development)").clicked() {
                next_state.set(AppState::List);
            }
        });
    });
}

pub fn cleanup(_commands: Commands) {
    println!("login cleanup");
}