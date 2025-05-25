use crate::pages::components::egui_common;
use crate::state::AppState;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::css::*;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
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
    position: Vec<(Vec2, Vec2)>,
}

#[derive(Resource)]
pub struct DetailData {
    image_entity: Entity,
    cursor_posision: Option<Vec2>,
    start_position: Option<Vec2>,
    rectangles: Vec<Rectangle>,
    selected_rectangles_index: Option<usize>,
    selected_class: usize,
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
        cursor_posision: None,
        start_position: None,
        rectangles: Vec::new(),
        selected_rectangles_index: None,
        selected_class: 1,
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

#[allow(clippy::too_many_arguments)]
pub fn update(
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut gizmos: Gizmos,
    mut selected_rect_gizmos: Gizmos<SelectedRect>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut detail_data: ResMut<DetailData>,
    mut contexts: EguiContexts,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut cursor_moved_events: EventReader<CursorMoved>,
) {
    // egui is consuming mouse input
    let egui_input_use = contexts.ctx_mut().wants_pointer_input();

    let (camera, camera_transform) = cameras.single().unwrap();
    for event in cursor_moved_events.read() {
        // move to world pos
        detail_data.cursor_posision = camera
            .viewport_to_world_2d(camera_transform, event.position)
            .ok();
    }

    if !egui_input_use {
        let selected_class = detail_data.selected_class;
        for event in mouse_button_input_events.read() {
            if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
                detail_data.start_position = detail_data.cursor_posision;
            }

            if detail_data.start_position.is_some()
                && event.button == MouseButton::Left
                && event.state == ButtonState::Released
            {
                // Store the positions in temporary variables to avoid simultaneous borrows
                let start_pos = detail_data.start_position.unwrap();
                let end_pos = detail_data.cursor_posision.unwrap();

                // Now push to rectangles using the temporary variables
                detail_data.rectangles.push(Rectangle {
                    class: selected_class,
                    position: vec![(start_pos, end_pos)],
                });
                detail_data.start_position = None;
            }
        }
    }

    // dragging rect
    if detail_data.start_position.is_some() {
        let start_pos = detail_data.start_position.unwrap();
        let end_pos = detail_data.cursor_posision.unwrap();
        gizmos.rect_2d(
            (start_pos + end_pos) / 2.0,
            end_pos - start_pos,
            rect_color(detail_data.selected_class),
        );
    }

    // Draw all stored rectangles
    let current_selected = detail_data.selected_rectangles_index;
    for (index, rect) in detail_data.rectangles.iter().enumerate() {
        // Get the first position pair from the rectangle
        if let Some((start_pos, end_pos)) = rect.position.first() {
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
        .default_width(150.0)
        .width_range(80.0..=500.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Left Panel");
            });
        });

    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(150.0)
        .width_range(80.0..=500.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Right Panel");
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
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
        });
}

pub fn cleanup(mut commands: Commands, detail_data: Res<DetailData>) {
    println!("detail cleanup");
    commands.entity(detail_data.image_entity).despawn();
}
