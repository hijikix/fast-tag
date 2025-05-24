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

#[derive(Resource)]
pub struct DetailData {
    image_entity: Entity,
    cursor_posision: Option<Vec2>,
    start_position: Option<Vec2>,
    rectangles: Vec<(Vec2, Vec2)>,
    selected_rectangles_index: Option<usize>,
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
    });
}

pub fn update(
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut gizmos: Gizmos,
    mut selected_rect_gizmos: Gizmos<SelectedRect>,
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

    for event in mouse_button_input_events.read() {
        info!("{:?}", event);
        if !egui_input_use
            && event.button == MouseButton::Left
            && event.state == ButtonState::Pressed
        {
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
            detail_data.rectangles.push((start_pos, end_pos));
            detail_data.start_position = None;
        }
    }

    // dragging rect
    if detail_data.start_position.is_some() {
        let start_pos = detail_data.start_position.unwrap();
        let end_pos = detail_data.cursor_posision.unwrap();
        gizmos.rect_2d((start_pos + end_pos) / 2.0, end_pos - start_pos, RED);
    }

    // Draw all stored rectangles
    let current_selected = detail_data.selected_rectangles_index;
    for (index, (start_pos, end_pos)) in detail_data.rectangles.iter().enumerate() {
        let is_selected = current_selected == Some(index);
        if is_selected {
            selected_rect_gizmos.rect_2d((start_pos + end_pos) / 2.0, end_pos - start_pos, RED);
        } else {
            gizmos.rect_2d((start_pos + end_pos) / 2.0, end_pos - start_pos, RED);
        }
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
