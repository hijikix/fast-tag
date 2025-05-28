use bevy::prelude::*;
use bevy_egui::egui::scroll_area::ScrollBarVisibility;
use bevy_egui::{EguiContexts, egui};
use crate::rectangle::Rectangle;

pub fn render_rectangle_list(
    ui: &mut egui::Ui,
    rectangles: &[Rectangle],
    selected_index: Option<usize>,
) -> Option<usize> {
    let mut new_selected = selected_index;
    
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            for (index, _rect) in rectangles.iter().enumerate() {
                let is_selected = selected_index == Some(index);
                let item = format!("element {index}");
                if ui.selectable_label(is_selected, item).clicked() {
                    new_selected = Some(index);
                }
            }
        });
    
    new_selected
}

pub fn render_rectangle_editor(
    ui: &mut egui::Ui,
    rectangles: &mut [Rectangle],
    selected_index: Option<usize>,
) {
    if let Some(index) = selected_index {
        if let Some(rectangle) = rectangles.get_mut(index) {
            ui.label(format!("element {}", index));
            ui.label(format!("Class: {}", rectangle.class));

            let (bottom_left, top_right) = &mut rectangle.position;
            ui.separator();

            let mut changed = false;

            ui.label("Max Position:");
            let mut x = top_right.x;
            let mut y = top_right.y;

            ui.horizontal(|ui| {
                ui.label("X:");
                if ui.add(egui::DragValue::new(&mut x).speed(1.0)).changed() {
                    top_right.x = x;
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Y:");
                if ui.add(egui::DragValue::new(&mut y).speed(1.0)).changed() {
                    top_right.y = y;
                    changed = true;
                }
            });

            ui.separator();

            ui.label("Min Position:");
            let mut x = bottom_left.x;
            let mut y = bottom_left.y;

            ui.horizontal(|ui| {
                ui.label("X:");
                if ui.add(egui::DragValue::new(&mut x).speed(1.0)).changed() {
                    bottom_left.x = x;
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Y:");
                if ui.add(egui::DragValue::new(&mut y).speed(1.0)).changed() {
                    bottom_left.y = y;
                    changed = true;
                }
            });

            if changed {
                rectangle.normalize_position();
            }
        } else {
            ui.label("Selected rectangle not found");
        }
    } else {
        ui.label("No rectangle selected");
    }
}

pub fn render_side_panels(
    contexts: &mut EguiContexts,
    rectangles: &mut [Rectangle],
    selected_index: &mut Option<usize>,
) {
    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(250.0)
        .width_range(80.0..=500.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Left Panel");
            });
        });

    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(250.0)
        .width_range(80.0..=500.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Right Panel");
            });

            ui.vertical(|ui| {
                let available_height = ui.available_height();
                ui.allocate_ui(
                    egui::Vec2::new(ui.available_width(), available_height * 0.5),
                    |ui| {
                        *selected_index = render_rectangle_list(ui, rectangles, *selected_index);
                        ui.allocate_space(ui.available_size());
                    },
                );
                ui.separator();
            });
        });
}

pub fn render_rectangle_editor_window(
    contexts: &mut EguiContexts,
    rectangles: &mut [Rectangle],
    selected_index: Option<usize>,
) {
    egui::Window::new("Selected").show(contexts.ctx_mut(), |ui| {
        render_rectangle_editor(ui, rectangles, selected_index);
    });
}