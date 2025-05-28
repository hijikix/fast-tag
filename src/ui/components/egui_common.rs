use crate::app::state::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

pub fn ui_top_panel(
    contexts: &mut EguiContexts,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    egui::TopBottomPanel::top("top_panel").show(contexts.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            egui::widgets::global_theme_preference_switch(ui);

            ui.separator();

            if ui
                .selectable_label(*current_state == AppState::List, "✨ List")
                .clicked()
            {
                next_state.set(AppState::List)
            }

            if ui
                .selectable_label(*current_state == AppState::Detail, "🕑 Detail")
                .clicked()
            {
                next_state.set(AppState::Detail)
            }
        });
    });
}
