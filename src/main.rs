use bevy::prelude::*;

mod state;
use bevy_egui::{EguiContextPass, EguiPlugin};
use state::AppState;

mod pages {
    pub mod components;
    pub mod detail;
    pub mod list;
}

use pages::{detail, list};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .init_state::<AppState>()
        .add_systems(Startup, setup)
        // list page
        .add_systems(OnEnter(AppState::List), list::setup)
        .add_systems(Update, list::update.run_if(in_state(AppState::List)))
        .add_systems(
            EguiContextPass,
            list::ui_system.run_if(in_state(AppState::List)),
        )
        .add_systems(OnExit(AppState::List), list::cleanup)
        // detail page
        .init_resource::<detail::Parameters>()
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
