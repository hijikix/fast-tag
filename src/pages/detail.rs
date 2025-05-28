use crate::ui::components::egui_common;
use crate::app::state::AppState;
use crate::io::image_loader;
use crate::core::rectangle::{Rectangle, rect_color};
use crate::core::interactions::{InteractionMode, ResizingHandler, GrabbingHandler, DrawingHandler, key_code_to_class};
use crate::core::camera_controls::CameraController;
use crate::ui::detail_ui;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

#[derive(Resource, Default)]
pub struct Parameters {
    pub url: String,
}





#[derive(Resource)]
pub struct DetailData {
    image_entity: Entity,
    cursor_position: Option<Vec2>,
    selected_class: usize,
    camera_controller: CameraController,
}

#[derive(Resource, Default)]
pub struct Rectangles(pub Vec<Rectangle>);

#[derive(Resource, Default)]
pub struct SelectedRectangleIndex(pub Option<usize>);

#[derive(Resource, Default)]
pub struct InteractionState {
    mode: InteractionMode,
}

#[derive(Resource, Default)]
pub struct InteractionHandlers {
    resizing: ResizingHandler,
    grabbing: GrabbingHandler, 
    drawing: DrawingHandler,
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
    let image_entity = match image_loader::spawn_image_sprite(&mut commands, &mut images, &params.url) {
        Ok(entity) => entity,
        Err(e) => {
            eprintln!("load_image error: {}", e);
            return;
        }
    };

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
        selected_class: 1,
        cursor_position: None,
        camera_controller: CameraController::default(),
    });
    
    commands.insert_resource(Rectangles::default());
    commands.insert_resource(SelectedRectangleIndex::default());
    commands.insert_resource(InteractionState::default());
    commands.insert_resource(InteractionHandlers::default());
}









fn draw_rectangles(
    rectangles: &Rectangles,
    selected_index: &SelectedRectangleIndex,
    gizmos: &mut Gizmos,
    selected_rect_gizmos: &mut Gizmos<SelectedRect>,
) {
    let current_selected = selected_index.0;
    for (index, rect) in rectangles.0.iter().enumerate() {
        let is_selected = current_selected == Some(index);
        if is_selected {
            selected_rect_gizmos.rect_2d(
                rect.center(),
                rect.size(),
                rect_color(rect.class),
            );
        } else {
            gizmos.rect_2d(
                rect.center(),
                rect.size(),
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
    mut rectangles: ResMut<Rectangles>,
    mut selected_index: ResMut<SelectedRectangleIndex>,
    mut interaction_state: ResMut<InteractionState>,
    mut handlers: ResMut<InteractionHandlers>,
    mut egui_contexts: EguiContexts,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut camera_transforms: Query<&mut Transform, With<Camera>>,
) {
    let egui_input_use = egui_contexts.ctx_mut().wants_pointer_input();

    // Get cursor position in world coordinates
    let (camera, camera_transform) = cameras.single().unwrap();
    let window = q_window.single().unwrap();
    let cursor_position = window
        .cursor_position()
        .and_then(|pos| camera.viewport_to_world_2d(camera_transform, pos).ok());
    
    if cursor_position.is_some() {
        detail_data.cursor_position = cursor_position;
    }

    let mouse_events: Vec<MouseButtonInput> = mouse_button_input_events.read().cloned().collect();

    // Process interactions
    let cursor_pos = detail_data.cursor_position;
    let selected_class = detail_data.selected_class;
    
    handlers.resizing.process(
        &mut rectangles.0,
        cursor_pos,
        &mouse_events,
        &mut interaction_state.mode,
        &mut selected_index.0,
        &mut egui_contexts,
    );

    handlers.grabbing.process(
        &mut rectangles.0,
        cursor_pos,
        &mouse_events,
        &mut interaction_state.mode,
        &mut selected_index.0,
        &mut egui_contexts,
    );

    handlers.drawing.process(
        &mut rectangles.0,
        cursor_pos,
        &mouse_events,
        &mut interaction_state.mode,
        selected_class,
        egui_input_use,
        &mut gizmos,
    );

    draw_rectangles(&rectangles, &selected_index, &mut gizmos, &mut selected_rect_gizmos);

    // Camera controls  
    detail_data.camera_controller.process_zoom(
        &mut mouse_wheel_events,
        &mut camera_transforms,
        egui_input_use,
    );
    detail_data.camera_controller.process_panning(
        &mouse_events,
        &mut camera_transforms,
        q_window,
        egui_input_use,
    );

    // Handle keyboard input
    if let Some(class) = key_code_to_class(&keyboard) {
        detail_data.selected_class = class;
    }

    if keyboard.pressed(KeyCode::Backspace) {
        if let Some(idx) = selected_index.0 {
            if idx < rectangles.0.len() {
                rectangles.0.remove(idx);
                selected_index.0 = None;
            }
        }
    }

    if keyboard.pressed(KeyCode::Escape) {
        selected_index.0 = None;
        interaction_state.mode = InteractionMode::Default;
        handlers.resizing.clear();
        handlers.grabbing.clear();
        handlers.drawing.clear();
        detail_data.camera_controller.reset_panning();
    }
}

pub fn ui_system(
    mut contexts: EguiContexts,
    current_state: Res<State<AppState>>,
    next_state: ResMut<NextState<AppState>>,
    mut rectangles: ResMut<Rectangles>,
    mut selected_index: ResMut<SelectedRectangleIndex>,
) {
    egui_common::ui_top_panel(&mut contexts, current_state, next_state);

    detail_ui::render_side_panels(
        &mut contexts,
        &mut rectangles.0,
        &mut selected_index.0,
    );

    detail_ui::render_rectangle_editor_window(
        &mut contexts,
        &mut rectangles.0,
        selected_index.0,
    );
}

pub fn cleanup(mut commands: Commands, detail_data: Res<DetailData>) {
    println!("detail cleanup");
    commands.entity(detail_data.image_entity).despawn();
}
