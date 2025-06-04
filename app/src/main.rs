use bevy::prelude::*;

mod app;
mod ui;
mod core;
mod io;
mod auth;
mod sync;
use bevy_egui::{EguiPlugin, EguiContexts, egui};
use app::state::AppState;
use auth::{AuthState, UserState, ProjectsState};

mod pages {
    pub mod detail;
    pub mod tasks;
    pub mod login;
    pub mod projects;
    pub mod project_settings;
}

use pages::{
    detail::DetailPlugin,
    tasks::TasksPlugin,
    login::LoginPlugin,
    projects::ProjectsPlugin,
    project_settings::ProjectSettingsPlugin,
};


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .init_state::<AppState>()
        .init_resource::<AuthState>()
        .init_resource::<UserState>()
        .init_resource::<ProjectsState>()
        .add_plugins(sync::SyncPlugin)
        .add_plugins(LoginPlugin)
        .add_plugins(TasksPlugin)
        .add_plugins(ProjectsPlugin)
        .add_plugins(ProjectSettingsPlugin)
        .add_plugins(DetailPlugin)
        .add_systems(Startup, (setup, setup_fonts))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_fonts(mut contexts: EguiContexts) {
    let mut fonts = egui::FontDefinitions::default();
    
    fonts.font_data.insert(
        "noto_sans_jp".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/NotoSansJP-Regular.ttf")).into(),
    );
    
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "noto_sans_jp".to_owned());
    
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("noto_sans_jp".to_owned());
    
    contexts.ctx_mut().set_fonts(fonts);
}
