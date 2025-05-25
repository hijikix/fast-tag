use crate::pages::components::egui_common;
use crate::state::AppState;
use bevy::color::palettes::css::*;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::{asset::RenderAssetUsages, window::PrimaryWindow};
use bevy_egui::egui::scroll_area::ScrollBarVisibility;
use bevy_egui::{
    EguiContexts,
    egui::{self},
};

#[derive(Resource, Default)]
pub struct Parameters {
    pub url: String,
}

fn load_image(url: &str) -> Result<image::DynamicImage, image::ImageError> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let image_bytes = rt.block_on(async {
        let response = match reqwest::get(url).await {
            Ok(response) => response,
            Err(e) => {
                eprintln!("image download error: {}", e);
                return None;
            }
        };

        match response.bytes().await {
            Ok(bytes) => Some(bytes.to_vec()),
            Err(e) => {
                eprintln!("image bytes error: {}", e);
                None
            }
        }
    });

    if let Some(bytes) = image_bytes {
        let image = image::load_from_memory(&bytes)?;
        println!("image loaded!!!");
        return Ok(image);
    }

    Err(image::ImageError::Unsupported(
        image::error::UnsupportedError::from_format_and_kind(
            image::error::ImageFormatHint::Unknown,
            image::error::UnsupportedErrorKind::Format(image::error::ImageFormatHint::Unknown),
        ),
    ))
}

pub struct Rectangle {
    class: usize,
    position: (Vec2, Vec2),
}

impl Rectangle {
    /// Normalize the rectangle coordinates to ensure consistent min/max ordering
    /// Returns (min_pos, max_pos) where min_pos has smaller x,y and max_pos has larger x,y
    fn normalize_position(&mut self) {
        let (pos1, pos2) = &mut self.position;
        let min_x = pos1.x.min(pos2.x);
        let max_x = pos1.x.max(pos2.x);
        let min_y = pos1.y.min(pos2.y);
        let max_y = pos1.y.max(pos2.y);

        *pos1 = Vec2::new(min_x, min_y);
        *pos2 = Vec2::new(max_x, max_y);
    }
}

#[derive(PartialEq)]
enum Mode {
    Default,
    Resizing,
    Drawing,
    Grabbing,
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum Corner {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

#[derive(Resource)]
pub struct DetailData {
    image_entity: Entity,
    cursor_posision: Option<Vec2>,

    rectangles: Vec<Rectangle>,
    selected_rectangles_index: Option<usize>,
    selected_class: usize,

    mode: Mode,

    drawing_start_position: Option<Vec2>,

    grabbing_corner_rectangles_index: Option<usize>,
    grabbing_corner: Option<Corner>,

    grabbing_rectangles_index: Option<usize>,
    grabbing_start_position: Option<Vec2>,
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelectedRect;

pub fn setup(
    mut commands: Commands,
    params: Res<Parameters>,
    mut images: ResMut<Assets<Image>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    println!("detail setup");

    // load image
    println!("url {:?}", params.url);
    let dynamic_image = match load_image(&params.url) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("load_image error: {}", e);
            return;
        }
    };
    let image = Image::from_dynamic(dynamic_image, true, RenderAssetUsages::default());
    let image_handle = images.add(image);
    let image_entity = commands.spawn(Sprite::from_image(image_handle)).id();

    // gizmo config
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 3.;
    let (selected_rect_config, _) = config_store.config_mut::<SelectedRect>();
    selected_rect_config.line.width = 5.;
    selected_rect_config.line.style = GizmoLineStyle::Dashed {
        gap_scale: 3.0,
        line_scale: 3.0,
    };

    // add resource
    commands.insert_resource(DetailData {
        image_entity,
        mode: Mode::Default,
        rectangles: Vec::new(),
        selected_rectangles_index: None,
        selected_class: 1,
        cursor_posision: None,
        drawing_start_position: None,
        grabbing_corner_rectangles_index: None,
        grabbing_corner: None,
        grabbing_rectangles_index: None,
        grabbing_start_position: None,
    });
}

fn rect_color(class: usize) -> impl Into<Color> {
    if class == 1 {
        return RED;
    } else if class == 2 {
        return BLUE;
    } else if class == 3 {
        return GREEN;
    } else if class == 4 {
        return YELLOW;
    } else if class == 5 {
        return PURPLE;
    } else if class == 6 {
        return AQUA;
    } else if class == 7 {
        return BROWN;
    } else if class == 8 {
        return NAVY;
    } else if class == 9 {
        return LIME;
    }
    return BLACK;
}

fn key_code_to_class(keyboard: Res<ButtonInput<KeyCode>>) -> Option<usize> {
    if keyboard.pressed(KeyCode::Digit1) {
        return Some(1);
    }
    if keyboard.pressed(KeyCode::Digit2) {
        return Some(2);
    }
    if keyboard.pressed(KeyCode::Digit3) {
        return Some(3);
    }
    if keyboard.pressed(KeyCode::Digit4) {
        return Some(4);
    }
    if keyboard.pressed(KeyCode::Digit5) {
        return Some(5);
    }
    if keyboard.pressed(KeyCode::Digit6) {
        return Some(6);
    }
    if keyboard.pressed(KeyCode::Digit7) {
        return Some(7);
    }
    if keyboard.pressed(KeyCode::Digit8) {
        return Some(8);
    }
    if keyboard.pressed(KeyCode::Digit9) {
        return Some(9);
    }
    return None;
}

fn is_cursor_posision_corner(
    cursor_posision: Option<Vec2>,
    position: &(Vec2, Vec2),
) -> (bool, Option<Corner>) {
    // If cursor position is None, there can't be an overlap
    let cursor_pos = match cursor_posision {
        Some(pos) => pos,
        None => return (false, None),
    };

    // Margin for detection (pixels)
    let margin = 5.0;

    // Check the single rectangle position tuple
    let (pos1, pos2) = position;
    // Normalize coordinates to ensure consistent min/max ordering
    let min_x = pos1.x.min(pos2.x);
    let max_x = pos1.x.max(pos2.x);
    let min_y = pos1.y.min(pos2.y);
    let max_y = pos1.y.max(pos2.y);

    let bottom_left = Vec2::new(min_x, min_y);
    let top_right = Vec2::new(max_x, max_y);
    let bottom_right = Vec2::new(max_x, min_y);
    let top_left = Vec2::new(min_x, max_y);

    if (cursor_pos - bottom_left).length() <= margin {
        return (true, Some(Corner::BottomLeft));
    }

    if (cursor_pos - top_right).length() <= margin {
        return (true, Some(Corner::TopRight));
    }

    if (cursor_pos - bottom_right).length() <= margin {
        return (true, Some(Corner::BottomRight));
    }

    if (cursor_pos - top_left).length() <= margin {
        return (true, Some(Corner::TopLeft));
    }

    // No corner overlap found
    (false, None)
}

fn is_cursor_posision_overlap(cursor_posision: Option<Vec2>, position: &(Vec2, Vec2)) -> bool {
    // If cursor position is None, there can't be an overlap
    let cursor_pos = match cursor_posision {
        Some(pos) => pos,
        None => return false,
    };

    // Margin for detection (pixels)
    let margin = 5.0;

    // Check the single rectangle position tuple
    let (top_left, bottom_right) = position;
    // Ensure correct bounds by finding min and max coordinates
    let min_x = top_left.x.min(bottom_right.x);
    let max_x = top_left.x.max(bottom_right.x);
    let min_y = top_left.y.min(bottom_right.y);
    let max_y = top_left.y.max(bottom_right.y);

    // Check if cursor position is on the edge of the rectangle (within margin)
    // Check if cursor is near the horizontal edges
    let near_horizontal_edge = (cursor_pos.y >= min_y - margin && cursor_pos.y <= min_y + margin)
        || (cursor_pos.y >= max_y - margin && cursor_pos.y <= max_y + margin);

    // Check if cursor is near the vertical edges
    let near_vertical_edge = (cursor_pos.x >= min_x - margin && cursor_pos.x <= min_x + margin)
        || (cursor_pos.x >= max_x - margin && cursor_pos.x <= max_x + margin);

    // Check if cursor is within the rectangle's x-range (for horizontal edges)
    let within_x_range = cursor_pos.x >= min_x && cursor_pos.x <= max_x;

    // Check if cursor is within the rectangle's y-range (for vertical edges)
    let within_y_range = cursor_pos.y >= min_y && cursor_pos.y <= max_y;

    // Return true if cursor is on any edge
    if (near_horizontal_edge && within_x_range) || (near_vertical_edge && within_y_range) {
        return true;
    }

    // No overlap found
    false
}

fn prosess_resizing_mode(
    detail_data: &mut ResMut<DetailData>,
    mouse_button_input_events: &mut EventReader<MouseButtonInput>,
    cursor_posision: &Option<Vec2>,
    egui_contexts: &mut EguiContexts,
) {
    // select overlap
    let mut hovering_index = None;
    let mut corner_index = None;
    for (index, rect) in detail_data.rectangles.iter().enumerate() {
        let (hovering, corner_idx) =
            is_cursor_posision_corner(detail_data.cursor_posision, &rect.position);
        if hovering {
            hovering_index = Some(index);
            corner_index = corner_idx; // corner_idx is already an Option<Corner>
            break;
        }
    }

    // use egui icon chenge method
    // bevy icon change method not work!!
    let ctx = egui_contexts.ctx_mut();

    if detail_data.mode == Mode::Default && hovering_index.is_some() {
        ctx.set_cursor_icon(egui::CursorIcon::Grab);

        // Store corner_index in a local variable to avoid moving it in the loop
        let corner = corner_index;

        for event in mouse_button_input_events.read() {
            if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
                detail_data.grabbing_corner_rectangles_index = hovering_index;
                detail_data.grabbing_corner = corner;
                detail_data.mode = Mode::Resizing;
            }
        }
    }

    if detail_data.mode == Mode::Resizing {
        // move grabbing rect
        // Extract the values we need before the mutable borrow
        let current_pos = cursor_posision.unwrap();

        // Extract and copy the values we need before the mutable borrow
        let rectangle_index = *detail_data
            .grabbing_corner_rectangles_index
            .as_ref()
            .unwrap();
        let corner = *detail_data.grabbing_corner.as_ref().unwrap();

        if let Some(rectangle) = detail_data.rectangles.get_mut(rectangle_index) {
            let (start_pos, end_pos) = &mut rectangle.position;

            // Update only the specific corner based on corner_index
            match corner {
                Corner::BottomLeft => *start_pos = current_pos,
                Corner::BottomRight => {
                    *start_pos = Vec2::new(start_pos.x, current_pos.y);
                    *end_pos = Vec2::new(current_pos.x, end_pos.y);
                }
                Corner::TopLeft => {
                    *start_pos = Vec2::new(current_pos.x, start_pos.y);
                    *end_pos = Vec2::new(end_pos.x, current_pos.y);
                }
                Corner::TopRight => *end_pos = current_pos,
            }
        }
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);

        for event in mouse_button_input_events.read() {
            if event.button == MouseButton::Left && event.state == ButtonState::Released {
                // Normalize coordinates after resizing to ensure consistency
                if let Some(rectangle_index) = detail_data.grabbing_corner_rectangles_index {
                    if let Some(rectangle) = detail_data.rectangles.get_mut(rectangle_index) {
                        rectangle.normalize_position();
                    }
                }

                detail_data.selected_rectangles_index =
                    detail_data.grabbing_corner_rectangles_index;
                detail_data.grabbing_corner_rectangles_index = None;
                detail_data.grabbing_corner = None;
                detail_data.mode = Mode::Default;
            }
        }
    }
}

fn prosess_grabbing_mode(
    detail_data: &mut ResMut<DetailData>,
    mouse_button_input_events: &mut EventReader<MouseButtonInput>,
    cursor_posision: &Option<Vec2>,
    egui_contexts: &mut EguiContexts,
) {
    // select overlap
    let mut hovering_index = None;
    for (index, rect) in detail_data.rectangles.iter().enumerate() {
        if is_cursor_posision_overlap(detail_data.cursor_posision, &rect.position) {
            hovering_index = Some(index);
            break;
        }
    }

    // use egui icon chenge method
    // bevy icon change method not work!!
    let ctx = egui_contexts.ctx_mut();

    if detail_data.mode == Mode::Default && hovering_index.is_some() {
        ctx.set_cursor_icon(egui::CursorIcon::Grab);
        for event in mouse_button_input_events.read() {
            if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
                detail_data.grabbing_rectangles_index = hovering_index;
                detail_data.grabbing_start_position = *cursor_posision;
                detail_data.mode = Mode::Grabbing;
            }
        }
    }

    if detail_data.mode == Mode::Grabbing {
        // move grabbing rect
        if detail_data.grabbing_start_position.is_some() {
            // Extract the values we need before the mutable borrow
            let grab_pos = detail_data.grabbing_start_position.unwrap();
            let current_pos = cursor_posision.unwrap();
            let moved = current_pos - grab_pos;

            // Extract the index before the mutable borrow
            let rectangle_index = detail_data.grabbing_rectangles_index.unwrap();
            if let Some(rectangle) = detail_data.rectangles.get_mut(rectangle_index) {
                // Update both points in the tuple separately
                let (start_pos, end_pos) = &mut rectangle.position;
                *start_pos += moved;
                *end_pos += moved;
                detail_data.grabbing_start_position = Some(current_pos);
            }
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        for event in mouse_button_input_events.read() {
            if event.button == MouseButton::Left && event.state == ButtonState::Released {
                detail_data.selected_rectangles_index = detail_data.grabbing_rectangles_index;
                detail_data.grabbing_rectangles_index = None;
                detail_data.grabbing_start_position = None;
                detail_data.mode = Mode::Default;
            }
        }
    }
}

fn prosess_drawing_mode(
    detail_data: &mut ResMut<DetailData>,
    mouse_button_input_events: &mut EventReader<MouseButtonInput>,
    egui_input_use: bool,
    gizmos: &mut Gizmos,
) {
    if detail_data.mode == Mode::Default && !egui_input_use && detail_data.cursor_posision.is_some()
    {
        for event in mouse_button_input_events.read() {
            if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
                detail_data.drawing_start_position = detail_data.cursor_posision;
                detail_data.mode = Mode::Drawing;
            }
        }
    }

    if detail_data.mode == Mode::Drawing && !egui_input_use && detail_data.cursor_posision.is_some()
    {
        let selected_class = detail_data.selected_class;
        for event in mouse_button_input_events.read() {
            if detail_data.drawing_start_position.is_some()
                && event.button == MouseButton::Left
                && event.state == ButtonState::Released
            {
                // Store the positions in temporary variables to avoid simultaneous borrows
                let start_pos = detail_data.drawing_start_position.unwrap();
                let end_pos = detail_data.cursor_posision.unwrap();

                // Calculate bottom-left (min coordinates) and top-right (max coordinates)
                let bottom_left = Vec2::new(start_pos.x.min(end_pos.x), start_pos.y.min(end_pos.y));
                let top_right = Vec2::new(start_pos.x.max(end_pos.x), start_pos.y.max(end_pos.y));

                // Now push to rectangles using bottom-left and top-right coordinates
                detail_data.rectangles.push(Rectangle {
                    class: selected_class,
                    position: (bottom_left, top_right),
                });
                detail_data.drawing_start_position = None;
                detail_data.mode = Mode::Default;
            }
        }
    }

    // draw current drawing rectangle
    if detail_data.mode == Mode::Drawing
        && detail_data.drawing_start_position.is_some()
        && detail_data.cursor_posision.is_some()
    {
        let start_pos = detail_data.drawing_start_position.unwrap();
        let end_pos = detail_data.cursor_posision.unwrap();
        gizmos.rect_2d(
            (start_pos + end_pos) / 2.0,
            end_pos - start_pos,
            rect_color(detail_data.selected_class),
        );
    }
}

fn draw_rectangles(
    detail_data: &ResMut<DetailData>,
    gizmos: &mut Gizmos,
    selected_rect_gizmos: &mut Gizmos<SelectedRect>,
) {
    // Draw all stored rectangles
    let current_selected = detail_data.selected_rectangles_index;
    for (index, rect) in detail_data.rectangles.iter().enumerate() {
        // Get the position pair from the rectangle
        let (start_pos, end_pos) = &rect.position;
        let is_selected = current_selected == Some(index);
        if is_selected {
            selected_rect_gizmos.rect_2d(
                (start_pos + end_pos) / 2.0,
                end_pos - start_pos,
                rect_color(rect.class),
            );
        } else {
            gizmos.rect_2d(
                (start_pos + end_pos) / 2.0,
                end_pos - start_pos,
                rect_color(rect.class),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    cameras: Query<(&Camera, &GlobalTransform)>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut gizmos: Gizmos,
    mut selected_rect_gizmos: Gizmos<SelectedRect>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut detail_data: ResMut<DetailData>,
    mut egui_contexts: EguiContexts,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
) {
    // egui is consuming mouse input
    let egui_input_use = egui_contexts.ctx_mut().wants_pointer_input();

    // get cursor_posision
    let (camera, camera_transform) = cameras.single().unwrap();
    let window = q_window.single().unwrap();
    // move to world pos
    let cursor_posision = window
        .cursor_position()
        .and_then(|pos| camera.viewport_to_world_2d(camera_transform, pos).ok());
    if cursor_posision.is_some() {
        detail_data.cursor_posision = cursor_posision;
    }

    prosess_resizing_mode(
        &mut detail_data,
        &mut mouse_button_input_events,
        &cursor_posision,
        &mut egui_contexts,
    );

    prosess_grabbing_mode(
        &mut detail_data,
        &mut mouse_button_input_events,
        &cursor_posision,
        &mut egui_contexts,
    );

    prosess_drawing_mode(
        &mut detail_data,
        &mut mouse_button_input_events,
        egui_input_use,
        &mut gizmos,
    );

    draw_rectangles(&detail_data, &mut gizmos, &mut selected_rect_gizmos);

    // select class by numeric key
    if let Some(class) = key_code_to_class(keyboard) {
        detail_data.selected_class = class;
    }
}

pub fn ui_system(
    mut contexts: EguiContexts,
    current_state: Res<State<AppState>>,
    next_state: ResMut<NextState<AppState>>,
    mut detail_data: ResMut<DetailData>,
) {
    egui_common::ui_top_panel(&mut contexts, current_state, next_state);

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
                // half top
                ui.allocate_ui(
                    egui::Vec2::new(ui.available_width(), available_height * 0.5),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                            .show(ui, |ui| {
                                // Store the current selected index before the loop to avoid simultaneous borrows
                                let current_selected = detail_data.selected_rectangles_index;
                                let mut new_selected = current_selected;

                                // Iterate over rectangles without borrowing detail_data inside the loop
                                for (index, _item) in detail_data.rectangles.iter().enumerate() {
                                    let is_selected = current_selected == Some(index);
                                    let item = format!("element {index}");
                                    if ui.selectable_label(is_selected, item).clicked() {
                                        new_selected = Some(index);
                                    }
                                }

                                // Update the selected index after the loop
                                detail_data.selected_rectangles_index = new_selected;
                            });
                        ui.allocate_space(ui.available_size());
                    },
                );

                ui.separator();
            });
        });

    egui::Window::new("Selected").show(contexts.ctx_mut(), |ui| {
        if let Some(index) = detail_data.selected_rectangles_index {
            if let Some(rectangle) = detail_data.rectangles.get_mut(index) {
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

                // Normalize coordinates if any value was changed
                if changed {
                    rectangle.normalize_position();
                }
            } else {
                ui.label("Selected rectangle not found");
            }
        } else {
            ui.label("No rectangle selected");
        }
    });
}

pub fn cleanup(mut commands: Commands, detail_data: Res<DetailData>) {
    println!("detail cleanup");
    commands.entity(detail_data.image_entity).despawn();
}
