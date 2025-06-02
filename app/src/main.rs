use bevy::prelude::*;

mod app;
mod ui;
mod core;
mod io;
mod auth;
use bevy_egui::{EguiContextPass, EguiPlugin};
use app::state::AppState;
use auth::{AuthState, UserState, ProjectsState};

mod pages {
    pub mod detail;
    pub mod list;
    pub mod login;
    pub mod projects;
}

use pages::{
    detail::{self, SelectedRect},
    list,
    login,
    projects,
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
        .add_systems(Startup, setup)
        // login page
        .add_systems(OnEnter(AppState::Login), login::setup)
        .add_systems(Update, login::update.run_if(in_state(AppState::Login)))
        .add_systems(
            EguiContextPass,
            login::ui_system.run_if(in_state(AppState::Login)),
        )
        .add_systems(OnExit(AppState::Login), login::cleanup)
        // list page
        .add_systems(OnEnter(AppState::List), list::setup)
        .add_systems(Update, list::update.run_if(in_state(AppState::List)))
        .add_systems(
            EguiContextPass,
            list::ui_system.run_if(in_state(AppState::List)),
        )
        .add_systems(OnExit(AppState::List), list::cleanup)
        // projects page
        .add_systems(OnEnter(AppState::Projects), projects::setup)
        .add_systems(Update, projects::update.run_if(in_state(AppState::Projects)))
        .add_systems(
            EguiContextPass,
            projects::ui_system.run_if(in_state(AppState::Projects)),
        )
        .add_systems(OnExit(AppState::Projects), projects::cleanup)
        // detail page
        .init_gizmo_group::<SelectedRect>()
        .init_resource::<detail::Parameters>()
        .init_resource::<detail::Rectangles>()
        .init_resource::<detail::SelectedRectangleIndex>()
        .init_resource::<detail::InteractionState>()
        .init_resource::<detail::InteractionHandlers>()
        .add_systems(OnEnter(AppState::Detail), detail::setup)
        .add_systems(Update, detail::update.run_if(in_state(AppState::Detail)))
        .add_systems(
            EguiContextPass,
            detail::ui_system.run_if(in_state(AppState::Detail)),
        )
        .add_systems(OnExit(AppState::Detail), detail::cleanup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
