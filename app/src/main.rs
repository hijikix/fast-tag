use bevy::prelude::*;

mod app;
mod ui;
mod core;
mod io;
mod auth;
mod sync;
use bevy_egui::EguiPlugin;
use app::state::AppState;
use auth::{AuthState, UserState, ProjectsState};

mod pages {
    pub mod detail;
    pub mod list;
    pub mod login;
    pub mod projects;
    pub mod project_settings;
}

use pages::{
    detail::DetailPlugin,
    list::ListPlugin,
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
        .add_plugins(ListPlugin)
        .add_plugins(ProjectsPlugin)
        .add_plugins(ProjectSettingsPlugin)
        .add_plugins(DetailPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
