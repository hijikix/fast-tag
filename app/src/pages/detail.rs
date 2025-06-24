use crate::app::state::AppState;
use crate::core::camera_controls::CameraController;
use crate::core::commands::{Command, CommandHistory};
use crate::core::interactions::{
    DrawingHandler, GrabbingHandler, InteractionMode, ResizingHandler, key_code_to_class,
};
use crate::core::rectangle::{Rectangle, rect_color};
use crate::io::image_loader;
use crate::ui::components::egui_common;
use crate::ui::detail_ui;
use crate::api::categories::CategoriesApi;
use crate::api::annotations::AnnotationsApi;
pub use crate::api::categories::AnnotationCategory;
pub use crate::api::annotations::{AnnotationWithCategory, BoundingBox};
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::text::Text2d;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiContextPass};
use uuid::Uuid;

#[derive(Resource, Default)]
pub struct Parameters {
    pub url: String,
    pub task_id: Option<uuid::Uuid>,
    pub project_id: Option<uuid::Uuid>,
}

#[derive(Resource)]
pub struct DetailData {
    image_entity: Entity,
    image_dimensions: Vec2,
    cursor_position: Option<Vec2>,
    selected_class: usize,
    camera_controller: CameraController,
    text_entities: Vec<Entity>,
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
    mut annotation_state: ResMut<AnnotationState>,
    auth_state: Res<crate::auth::AuthState>,
) {
    println!("detail setup");

    // load image
    println!("url {:?}", params.url);
    let (image_entity, image_dimensions) =
        match image_loader::spawn_image_sprite(&mut commands, &mut images, &params.url) {
            Ok((entity, dimensions)) => {
                println!("Image loaded successfully with dimensions: {:?}", dimensions);
                (entity, dimensions)
            },
            Err(e) => {
                eprintln!("load_image error: {}", e);
                eprintln!("Failed to load image from URL: {}", params.url);
                // Create a placeholder entity even when image loading fails
                // This prevents the DetailData resource from not being created
                (commands.spawn(Sprite::default()).id(), Vec2::new(100.0, 100.0))
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

    // add resource - always create this even if image loading failed
    commands.insert_resource(DetailData {
        image_entity,
        image_dimensions,
        selected_class: 1,
        cursor_position: None,
        camera_controller: CameraController::default(),
        text_entities: Vec::new(),
    });

    commands.insert_resource(Rectangles::default());
    commands.insert_resource(SelectedRectangleIndex::default());
    commands.insert_resource(InteractionState::default());
    commands.insert_resource(InteractionHandlers::default());
    commands.insert_resource(CommandHistory::default());
    
    // Set current task and project IDs for annotation system
    if let Some(task_id) = params.task_id {
        annotation_state.current_task_id = Some(task_id);
    }
    if let Some(project_id) = params.project_id {
        annotation_state.current_project_id = Some(project_id);
        
        // Load categories for this project
        if let Some(token) = auth_state.get_jwt() {
            let categories_api = CategoriesApi::new();
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(categories_api.list_categories(&token, project_id)) {
                Ok(categories) => {
                    annotation_state.categories = categories.clone();
                    info!("Loaded categories for project: {}", project_id);
                    
                    // Automatically load existing annotations
                    if let Some(task_id) = params.task_id {
                        match annotation_client::load_annotations(project_id, task_id, token.to_string(), true) {
                            Ok(annotations) => {
                                info!("Automatically loaded {} annotations", annotations.len());
                                info!("Auto-loaded annotations JSON: {}", serde_json::to_string_pretty(&annotations).unwrap_or_else(|_| "Failed to serialize".to_string()));
                                
                                // Convert loaded annotations to rectangles
                                let mut loaded_rectangles = Vec::new();
                                for annotation in annotations {
                                    let x = annotation.bbox[0] as f32;
                                    let y = annotation.bbox[1] as f32;
                                    let width = annotation.bbox[2] as f32;
                                    let height = annotation.bbox[3] as f32;
                                    
                                    // Convert from COCO format (top-left origin) to Bevy format (center origin)
                                    let center_x = x + width / 2.0 - image_dimensions.x / 2.0;
                                    let center_y = image_dimensions.y / 2.0 - (y + height / 2.0);
                                    
                                    // Calculate corner positions for Rectangle
                                    let half_width = width / 2.0;
                                    let half_height = height / 2.0;
                                    let start = Vec2::new(center_x - half_width, center_y - half_height);
                                    let end = Vec2::new(center_x + half_width, center_y + half_height);
                                    
                                    // Find the class index from category_id
                                    let class = if let Some(cat_id) = annotation.category_id {
                                        if let Some(category_index) = categories.iter().position(|cat| cat.id == cat_id) {
                                            (category_index % 9) + 1  // Convert 0-based index to 1-based class (1-9)
                                        } else {
                                            1  // Default to class 1 if category not found
                                        }
                                    } else {
                                        1  // Default to class 1 if no category
                                    };
                                    
                                    let rect = Rectangle::new(class, start, end);
                                    loaded_rectangles.push(rect);
                                }
                                
                                // Update rectangles resource
                                commands.insert_resource(Rectangles(loaded_rectangles.clone()));
                                info!("Converted and loaded {} rectangles", loaded_rectangles.len());
                            }
                            Err(error) => {
                                // Don't treat this as fatal - annotations might not exist yet
                                info!("No existing annotations found or failed to load: {}", error);
                            }
                        }
                    }
                }
                Err(error) => {
                    error!("Failed to load categories: {}", error);
                }
            }
        } else {
            warn!("No JWT token available for loading categories");
        }
    }
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
        let color = rect_color(rect.class);
        
        // Draw rectangle
        if is_selected {
            selected_rect_gizmos.rect_2d(rect.center(), rect.size(), color);
        } else {
            gizmos.rect_2d(rect.center(), rect.size(), color);
        }
    }
}

fn update_text_entities(
    commands: &mut Commands,
    detail_data: &mut DetailData,
    rectangles: &Rectangles,
) {
    // Clear existing text entities
    for entity in detail_data.text_entities.drain(..) {
        commands.entity(entity).despawn();
    }
    
    // Create new text entities for each rectangle
    for (index, rect) in rectangles.0.iter().enumerate() {
        let top_left = Vec2::new(
            rect.position.0.x.min(rect.position.1.x),
            rect.position.0.y.max(rect.position.1.y)
        );
        let text_position = top_left + Vec2::new(15.0, -15.0);
        
        let text_entity = commands.spawn((
            Text2d::new(index.to_string()),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_translation(text_position.extend(10.0)),
        )).id();
        
        detail_data.text_entities.push(text_entity);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
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
    mut command_history: ResMut<CommandHistory>,
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
    
    // Track the number of rectangles before processing
    let rect_count_before = rectangles.0.len();

    handlers.resizing.process(
        &mut rectangles.0,
        cursor_pos,
        &mouse_events,
        &mut interaction_state.mode,
        &mut selected_index.0,
        &mut egui_contexts,
        &mut command_history,
    );

    handlers.grabbing.process(
        &mut rectangles.0,
        cursor_pos,
        &mouse_events,
        &mut interaction_state.mode,
        &mut selected_index.0,
        &mut egui_contexts,
        &mut command_history,
    );

    handlers.drawing.process(
        &mut rectangles.0,
        cursor_pos,
        &mouse_events,
        &mut interaction_state.mode,
        selected_class,
        egui_input_use,
        &mut gizmos,
        &mut command_history,
    );
    
    // Update text entities if rectangles have changed
    let rect_count_after = rectangles.0.len();
    if rect_count_before != rect_count_after || detail_data.text_entities.len() != rect_count_after {
        update_text_entities(&mut commands, &mut detail_data, &rectangles);
    }

    draw_rectangles(
        &rectangles,
        &selected_index,
        &mut gizmos,
        &mut selected_rect_gizmos,
    );

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
                let rectangle = rectangles.0[idx].clone();
                let command = Command::DeleteRectangle {
                    index: idx,
                    rectangle,
                };
                command.execute(&mut rectangles.0);
                command_history.push(command);
                selected_index.0 = None;
                update_text_entities(&mut commands, &mut detail_data, &rectangles);
            }
        }
    }

    // Handle undo/redo
    let modifier_pressed = if cfg!(target_os = "macos") {
        keyboard.pressed(KeyCode::SuperLeft) || keyboard.pressed(KeyCode::SuperRight)
    } else {
        keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight)
    };
    
    if modifier_pressed && keyboard.just_pressed(KeyCode::KeyZ) {
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            // Redo with Cmd+Shift+Z (macOS) or Ctrl+Shift+Z (Windows/Linux)
            if command_history.redo(&mut rectangles.0) {
                selected_index.0 = None;
                update_text_entities(&mut commands, &mut detail_data, &rectangles);
            }
        } else {
            // Undo with Cmd+Z (macOS) or Ctrl+Z (Windows/Linux)
            if command_history.undo(&mut rectangles.0) {
                selected_index.0 = None;
                update_text_entities(&mut commands, &mut detail_data, &rectangles);
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

#[allow(clippy::too_many_arguments)]
pub fn ui_system(
    mut commands: Commands,
    mut contexts: EguiContexts,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut rectangles: ResMut<Rectangles>,
    mut selected_index: ResMut<SelectedRectangleIndex>,
    mut detail_data: ResMut<DetailData>,
    mut annotation_state: ResMut<AnnotationState>,
    auth_state: Res<crate::auth::AuthState>,
    user_state: Res<crate::auth::UserState>,
    projects_state: Res<crate::auth::ProjectsState>,
) {
    egui_common::ui_top_panel(&mut contexts, current_state, &mut next_state);

    let rect_count_before = rectangles.0.len();
    detail_ui::render_side_panels_with_annotations(
        &mut contexts, 
        &mut rectangles.0, 
        &mut selected_index.0,
        &mut annotation_state,
        &auth_state,
        &user_state,
        &projects_state,
        detail_data.image_dimensions,
    );
    
    // Check if rectangles were sorted (order might have changed)
    if rect_count_before == rectangles.0.len() && rect_count_before > 0 {
        // Update text entities to reflect new order
        update_text_entities(&mut commands, &mut detail_data, &rectangles);
    }

    detail_ui::render_rectangle_editor_window(&mut contexts, &mut rectangles.0, selected_index.0);
}


pub fn cleanup(mut commands: Commands, detail_data: Res<DetailData>) {
    println!("detail cleanup");
    commands.entity(detail_data.image_entity).despawn();
    
    // Clean up text entities
    for entity in &detail_data.text_entities {
        commands.entity(*entity).despawn();
    }
    
    commands.remove_resource::<CommandHistory>();
}

// Annotation types and structures
#[derive(Resource, Default)]
pub struct AnnotationState {
    pub is_saving: bool,
    pub categories: Vec<AnnotationCategory>,
    pub current_task_id: Option<Uuid>,
    pub current_project_id: Option<Uuid>,
}

// API types are now re-exported at the top of the file

// Adapter functions to maintain existing interface while using new API modules
pub mod annotation_client {
    use super::*;

    pub fn save_annotations(
        project_id: Uuid,
        task_id: Uuid,
        bounding_boxes: Vec<BoundingBox>,
        token: String,
    ) -> Result<Vec<AnnotationWithCategory>, String> {
        let annotations_api = AnnotationsApi::new();
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;
        
        runtime.block_on(async {
            annotations_api.save_annotations(&token, project_id, task_id, &bounding_boxes).await
                .map_err(|e| e.to_string())
        })
    }

    pub fn load_annotations(
        project_id: Uuid,
        task_id: Uuid,
        token: String,
        latest_only: bool,
    ) -> Result<Vec<AnnotationWithCategory>, String> {
        let annotations_api = AnnotationsApi::new();
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;
        
        runtime.block_on(async {
            annotations_api.list_annotations_with_options(&token, project_id, task_id, latest_only).await
                .map_err(|e| e.to_string())
        })
    }

}



pub struct DetailPlugin;

impl Plugin for DetailPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<SelectedRect>()
           .init_resource::<Parameters>()
           .init_resource::<Rectangles>()
           .init_resource::<SelectedRectangleIndex>()
           .init_resource::<InteractionState>()
           .init_resource::<InteractionHandlers>()
           .init_resource::<AnnotationState>()
           .add_systems(OnEnter(AppState::Detail), setup)
           .add_systems(Update, update.run_if(in_state(AppState::Detail)))
           .add_systems(
               EguiContextPass,
               ui_system.run_if(in_state(AppState::Detail)),
           )
           .add_systems(OnExit(AppState::Detail), cleanup);
    }
}
