use bevy::prelude::*;
use bevy::window::PrimaryWindow;

mod api;
mod app;
mod auth;
mod core;
mod io;
mod sync;
mod ui;
use app::state::AppState;
use auth::{AuthState, ProjectsState, UserState};
use bevy_egui::{EguiContexts, EguiPlugin, egui};

mod pages {
    pub mod detail;
    pub mod login;
    pub mod project_settings;
    pub mod projects;
    pub mod tasks;
}

use pages::{
    detail::DetailPlugin, login::LoginPlugin, project_settings::ProjectSettingsPlugin,
    projects::ProjectsPlugin, tasks::TasksPlugin,
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
        .add_systems(Startup, (setup, setup_fonts, maximize_window))
        .add_plugins(sync::SyncPlugin)
        .add_plugins(LoginPlugin)
        .add_plugins(TasksPlugin)
        .add_plugins(ProjectsPlugin)
        .add_plugins(ProjectSettingsPlugin)
        .add_plugins(DetailPlugin)
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

fn maximize_window(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in windows.iter_mut() {
        window.set_maximized(true);
    }
}
