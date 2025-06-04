use bevy::prelude::*;
use bevy_egui::egui::scroll_area::ScrollBarVisibility;
use bevy_egui::{EguiContexts, egui};
use crate::core::rectangle::{Rectangle, rect_color};

pub fn render_rectangle_list(
    ui: &mut egui::Ui,
    rectangles: &mut Vec<Rectangle>,
    selected_index: Option<usize>,
) -> Option<usize> {
    let mut new_selected = selected_index;
    
    // Add sorting buttons
    ui.horizontal(|ui| {
        ui.label("Sort by:");
        if ui.button("X Asc").clicked() {
            rectangles.sort_by(|a, b| {
                let a_x = a.position.0.x.min(a.position.1.x);
                let b_x = b.position.0.x.min(b.position.1.x);
                a_x.partial_cmp(&b_x).unwrap()
            });
        }
        if ui.button("X Desc").clicked() {
            rectangles.sort_by(|a, b| {
                let a_x = a.position.0.x.min(a.position.1.x);
                let b_x = b.position.0.x.min(b.position.1.x);
                b_x.partial_cmp(&a_x).unwrap()
            });
        }
        if ui.button("Y Asc").clicked() {
            rectangles.sort_by(|a, b| {
                let a_y = a.position.0.y.min(a.position.1.y);
                let b_y = b.position.0.y.min(b.position.1.y);
                a_y.partial_cmp(&b_y).unwrap()
            });
        }
        if ui.button("Y Desc").clicked() {
            rectangles.sort_by(|a, b| {
                let a_y = a.position.0.y.min(a.position.1.y);
                let b_y = b.position.0.y.min(b.position.1.y);
                b_y.partial_cmp(&a_y).unwrap()
            });
        }
    });
    
    ui.separator();
    
    // Store drag source and drop destination
    let mut from = None;
    let mut to = None;
    
    let frame = egui::Frame::default().inner_margin(4.0);
    
    let (_, dropped_payload) = ui.dnd_drop_zone::<usize, ()>(frame, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                for (index, rect) in rectangles.iter().enumerate() {
                    let item_id = egui::Id::new(("rectangle_list", index));
                    let is_selected = selected_index == Some(index);
                    
                    // Get color for this rectangle
                    let color: Color = rect_color(rect.class).into();
                    let egui_color = egui::Color32::from_rgba_unmultiplied(
                        (color.to_srgba().red * 255.0) as u8,
                        (color.to_srgba().green * 255.0) as u8,
                        (color.to_srgba().blue * 255.0) as u8,
                        255,
                    );
                    
                    let response = ui
                        .dnd_drag_source(item_id, index, |ui| {
                            ui.horizontal(|ui| {
                                // Add color indicator
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        ui.cursor().min,
                                        egui::Vec2::new(4.0, ui.spacing().interact_size.y)
                                    ),
                                    0.0,
                                    egui_color,
                                );
                                ui.add_space(8.0);
                                
                                let item = format!("element {index}");
                                if ui.selectable_label(is_selected, item).clicked() {
                                    new_selected = Some(index);
                                }
                            });
                        })
                        .response;
                    
                    // Detect drops onto this item
                    if let (Some(pointer), Some(hovered_payload)) = (
                        ui.input(|i| i.pointer.interact_pos()),
                        response.dnd_hover_payload::<usize>(),
                    ) {
                        let rect = response.rect;
                        let stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
                        
                        let insert_row_idx = if *hovered_payload == index {
                            // Dragging onto ourselves
                            ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                            index
                        } else if pointer.y < rect.center().y {
                            // Above us
                            ui.painter().hline(rect.x_range(), rect.top(), stroke);
                            index
                        } else {
                            // Below us
                            ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                            index + 1
                        };
                        
                        if let Some(dragged_payload) = response.dnd_release_payload() {
                            // The user dropped onto this item
                            from = Some(dragged_payload);
                            to = Some(insert_row_idx);
                        }
                    }
                }
            });
    });
    
    if let Some(dragged_payload) = dropped_payload {
        // The user dropped onto the list area, but not on any specific item
        from = Some(dragged_payload);
        to = Some(rectangles.len()); // Insert at the end
    }
    
    // Perform the move if there was a drop
    if let (Some(from_idx), Some(mut to_idx)) = (from, to) {
        let from_idx = *from_idx;  // Dereference Arc<usize> to usize
        if from_idx != to_idx {
            // Adjust target index if moving down
            if from_idx < to_idx {
                to_idx -= 1;
            }
            
            let item = rectangles.remove(from_idx);
            rectangles.insert(to_idx, item);
            
            // Update selected index if needed
            if let Some(sel) = selected_index {
                if sel == from_idx {
                    new_selected = Some(to_idx);
                } else if from_idx < sel && sel <= to_idx {
                    new_selected = Some(sel - 1);
                } else if to_idx <= sel && sel < from_idx {
                    new_selected = Some(sel + 1);
                }
            }
        }
    }
    
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
    rectangles: &mut Vec<Rectangle>,
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

#[allow(clippy::ptr_arg)]
pub fn render_rectangle_editor_window(
    contexts: &mut EguiContexts,
    rectangles: &mut Vec<Rectangle>,
    selected_index: Option<usize>,
) {
    egui::Window::new("Selected").show(contexts.ctx_mut(), |ui| {
        render_rectangle_editor(ui, rectangles, selected_index);
    });
}