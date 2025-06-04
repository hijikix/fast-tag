use crate::ui::components::egui_common;
use crate::app::state::AppState;
use crate::auth::{AuthState, ProjectsState};
use crate::sync::{SyncState, SyncRequestEvent, SyncRequest, SyncCompletedEvent, SyncErrorEvent};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiContextPass, egui};
use uuid::Uuid;

#[derive(Resource, Default)]
pub struct Parameters {
    pub project_id: String,
}

#[derive(Component)]
pub struct SaveProjectTask {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Component)]
pub struct DeleteProjectTask {
    pub project_id: String,
}

#[derive(Resource, Default)]
pub struct ProjectSettingsPageData {
    pub selected_project_id: Option<String>,
    pub project_name: String,
    pub project_description: String,
    pub is_editing: bool,
    pub save_error: Option<String>,
    pub is_saving: bool,
    pub is_deleting: bool,
    pub delete_error: Option<String>,
    pub show_delete_confirmation: bool,
    pub sync_status_message: Option<String>,
    pub sync_error_message: Option<String>,
}

pub fn setup(
    mut commands: Commands,
    projects_state: Res<ProjectsState>,
    parameters: Option<Res<Parameters>>,
) {
    println!("project_settings setup");
    
    let mut page_data = ProjectSettingsPageData::default();
    
    // Initialize with project from parameters if available, otherwise use first project
    if let Some(params) = parameters {
        if let Some(project) = projects_state.projects.iter().find(|p| p.id == params.project_id) {
            page_data.selected_project_id = Some(project.id.clone());
            page_data.project_name = project.name.clone();
            page_data.project_description = project.description.clone().unwrap_or_default();
        }
    } else if let Some(project) = projects_state.projects.first() {
        page_data.selected_project_id = Some(project.id.clone());
        page_data.project_name = project.name.clone();
        page_data.project_description = project.description.clone().unwrap_or_default();
    }
    
    commands.insert_resource(page_data);
}

pub fn update() {
    // No update logic needed for project settings page currently
}

#[allow(clippy::too_many_arguments)]
pub fn ui_system(
    mut commands: Commands,
    mut contexts: EguiContexts,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut page_data: ResMut<ProjectSettingsPageData>,
    projects_state: Res<ProjectsState>,
    auth_state: Res<AuthState>,
    sync_state: Res<SyncState>,
    mut sync_request_events: EventWriter<SyncRequestEvent>,
) {
    egui_common::ui_top_panel(&mut contexts, current_state, &mut next_state);

    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Project Settings");
            ui.add_space(10.0);
        });

        if projects_state.projects.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No projects available");
                ui.label("Go to Projects page to create a project first.");
            });
            return;
        }

        ui.separator();

        if let Some(project_id) = page_data.selected_project_id.clone() {
            if let Some(project) = projects_state.projects.iter().find(|p| p.id == project_id).cloned() {
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong("Project Details");
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if page_data.is_editing {
                                    let can_save = !page_data.project_name.trim().is_empty() && !page_data.is_saving;
                                    
                                    if ui.add_enabled(can_save, egui::Button::new("💾 Save")).clicked() {
                                        page_data.is_saving = true;
                                        page_data.save_error = None;
                                        
                                        // Spawn task to save project
                                        commands.spawn(SaveProjectTask {
                                            project_id: project_id.clone(),
                                            name: page_data.project_name.clone(),
                                            description: if page_data.project_description.is_empty() { 
                                                None 
                                            } else { 
                                                Some(page_data.project_description.clone()) 
                                            },
                                        });
                                    }
                                    
                                    if ui.button("❌ Cancel").clicked() {
                                        page_data.project_name = project.name.clone();
                                        page_data.project_description = project.description.clone().unwrap_or_default();
                                        page_data.is_editing = false;
                                        page_data.save_error = None;
                                    }
                                    
                                    if page_data.is_saving {
                                        ui.add(egui::Spinner::new());
                                    }
                                } else if ui.button("Edit").clicked() {
                                    page_data.is_editing = true;
                                }
                            });
                        });
                        
                        ui.separator();
                        
                        ui.add_space(10.0);
                        
                        // Project name
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            if page_data.is_editing {
                                ui.text_edit_singleline(&mut page_data.project_name);
                            } else {
                                ui.label(&project.name);
                            }
                        });
                        
                        ui.add_space(5.0);
                        
                        // Project description
                        ui.horizontal(|ui| {
                            ui.label("Description:");
                            if page_data.is_editing {
                                ui.vertical(|ui| {
                                    ui.text_edit_multiline(&mut page_data.project_description);
                                });
                            } else {
                                ui.label(project.description.as_deref().unwrap_or("No description"));
                            }
                        });
                        
                        ui.add_space(5.0);
                        
                        // Project metadata
                        ui.horizontal(|ui| {
                            ui.label("Created:");
                            ui.label(format_date(&project.created_at));
                        });
                        
                        // Show save error
                        if let Some(error) = &page_data.save_error {
                            ui.add_space(10.0);
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                        }
                    });
                });
                
                ui.add_space(20.0);
                
                // Storage Sync section
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.strong("Storage Sync");
                        ui.separator();
                        
                        ui.horizontal(|ui| {
                            ui.label("Sync files from storage to create annotation tasks.");
                        });
                        
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            let is_syncing = sync_state.is_syncing;
                            
                            if ui.add_enabled(!is_syncing, egui::Button::new("🔄 Start Sync")).clicked() {
                                if let Ok(project_uuid) = Uuid::parse_str(&project_id) {
                                    if let Some(jwt) = auth_state.get_jwt() {
                                        sync_request_events.write(SyncRequestEvent {
                                            project_id: project_uuid,
                                            request: SyncRequest {
                                                prefix: None,
                                                file_extensions: Some(vec!["jpg".to_string(), "jpeg".to_string(), "png".to_string()]),
                                                overwrite_existing: Some(false),
                                            },
                                            token: jwt.clone(),
                                        });
                                    }
                                    page_data.sync_status_message = Some("Starting sync...".to_string());
                                    page_data.sync_error_message = None;
                                }
                            }
                            
                            if is_syncing {
                                ui.add(egui::Spinner::new());
                                ui.label("Syncing...");
                                
                                if let Some(progress) = &sync_state.progress {
                                    ui.label(format!(
                                        "Progress: {} / {} files",
                                        progress.processed_files,
                                        progress.total_files
                                    ));
                                }
                            }
                        });
                        
                        if let Some(msg) = &page_data.sync_status_message {
                            ui.add_space(5.0);
                            ui.colored_label(egui::Color32::GREEN, msg);
                        }
                        
                        if let Some(err) = &page_data.sync_error_message {
                            ui.add_space(5.0);
                            ui.colored_label(egui::Color32::RED, err);
                        }
                    });
                });
                
                ui.add_space(20.0);
                
                // Danger zone
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.colored_label(egui::Color32::RED, "⚡ Danger Zone");
                        ui.separator();
                        
                        ui.horizontal(|ui| {
                            if ui.button("❌ Delete Project").clicked() {
                                page_data.show_delete_confirmation = true;
                            }
                            ui.label("This action cannot be undone.");
                        });
                    });
                });
            }
        }
    });
    
    // Delete confirmation dialog
    if page_data.show_delete_confirmation {
        egui::Window::new("⚠️ Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Are you sure you want to delete this project?");
                    ui.add_space(5.0);
                    ui.colored_label(egui::Color32::RED, "This action cannot be undone!");
                    
                    if let Some(project_id) = &page_data.selected_project_id {
                        if let Some(project) = projects_state.projects.iter().find(|p| &p.id == project_id) {
                            ui.add_space(10.0);
                            ui.strong(format!("Project: {}", project.name));
                        }
                    }
                    
                    ui.add_space(15.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            page_data.show_delete_confirmation = false;
                        }
                        
                        ui.add_space(10.0);
                        
                        let can_delete = !page_data.is_deleting;
                        if ui.add_enabled(can_delete, egui::Button::new("❌ Delete").fill(egui::Color32::from_rgb(220, 53, 69))).clicked() {
                            page_data.is_deleting = true;
                            page_data.delete_error = None;
                            page_data.show_delete_confirmation = false;
                            
                            if let Some(project_id) = &page_data.selected_project_id {
                                commands.spawn(DeleteProjectTask {
                                    project_id: project_id.clone(),
                                });
                            }
                        }
                        
                        if page_data.is_deleting {
                            ui.add(egui::Spinner::new());
                        }
                    });
                });
            });
    }
    
    // Show delete error
    if let Some(error) = page_data.delete_error.clone() {
        egui::Window::new("❌ Delete Error")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.colored_label(egui::Color32::RED, format!("Failed to delete project: {}", error));
                    
                    ui.add_space(10.0);
                    
                    if ui.button("OK").clicked() {
                        page_data.delete_error = None;
                    }
                });
            });
    }
}

fn format_date(date_str: &str) -> String {
    // Simple date formatting - just return the first 10 characters (YYYY-MM-DD)
    if date_str.len() >= 10 {
        date_str[..10].to_string()
    } else {
        date_str.to_string()
    }
}

pub fn cleanup(mut commands: Commands) {
    println!("project_settings cleanup");
    commands.remove_resource::<ProjectSettingsPageData>();
}

pub fn handle_save_project_task(
    mut commands: Commands,
    mut page_data: ResMut<ProjectSettingsPageData>,
    mut projects_state: ResMut<ProjectsState>,
    auth_state: Res<AuthState>,
    mut save_tasks: Query<(Entity, &SaveProjectTask)>,
) {
    for (entity, task) in save_tasks.iter_mut() {
        if let Some(jwt) = auth_state.get_jwt() {
            let jwt = jwt.clone();
            let project_id = task.project_id.clone();
            let name = task.name.clone();
            let description = task.description.clone();
            
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(crate::auth::update_project(&jwt, &project_id, &name, description.as_deref())) {
                Ok(updated_project) => {
                    // Update the project in projects_state
                    if let Some(project) = projects_state.projects.iter_mut().find(|p| p.id == updated_project.id) {
                        *project = updated_project.clone();
                        
                        // Update page data to reflect the changes
                        if Some(&project.id) == page_data.selected_project_id.as_ref() {
                            page_data.project_name = updated_project.name;
                            page_data.project_description = updated_project.description.unwrap_or_default();
                        }
                    }
                    
                    page_data.is_saving = false;
                    page_data.is_editing = false;
                    page_data.save_error = None;
                }
                Err(error) => {
                    page_data.save_error = Some(error);
                    page_data.is_saving = false;
                }
            }
        } else {
            page_data.save_error = Some("Not authenticated".to_string());
            page_data.is_saving = false;
        }
        commands.entity(entity).despawn();
    }
}

pub fn handle_sync_events(
    mut page_data: ResMut<ProjectSettingsPageData>,
    mut sync_completed_events: EventReader<SyncCompletedEvent>,
    mut sync_error_events: EventReader<SyncErrorEvent>,
) {
    for event in sync_completed_events.read() {
        page_data.sync_status_message = Some(format!(
            "Sync completed! Created {} tasks, skipped {} tasks.",
            event.response.tasks_created,
            event.response.tasks_skipped
        ));
        page_data.sync_error_message = None;
        
        if !event.response.errors.is_empty() {
            page_data.sync_error_message = Some(format!(
                "Completed with {} errors: {}",
                event.response.errors.len(),
                event.response.errors.join(", ")
            ));
        }
    }
    
    for event in sync_error_events.read() {
        page_data.sync_error_message = Some(format!("Sync failed: {}", event.error));
        page_data.sync_status_message = None;
    }
}

pub fn handle_delete_project_task(
    mut commands: Commands,
    mut page_data: ResMut<ProjectSettingsPageData>,
    mut projects_state: ResMut<ProjectsState>,
    mut next_state: ResMut<NextState<AppState>>,
    auth_state: Res<AuthState>,
    mut delete_tasks: Query<(Entity, &DeleteProjectTask)>,
) {
    for (entity, task) in delete_tasks.iter_mut() {
        if let Some(jwt) = auth_state.get_jwt() {
            let jwt = jwt.clone();
            let project_id = task.project_id.clone();
            
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(crate::auth::delete_project(&jwt, &project_id)) {
                Ok(()) => {
                    // Remove the project from projects_state
                    if let Some(project_id) = &page_data.selected_project_id {
                        projects_state.projects.retain(|p| &p.id != project_id);
                    }
                    
                    page_data.is_deleting = false;
                    page_data.delete_error = None;
                    
                    // Navigate back to projects list
                    next_state.set(AppState::Projects);
                }
                Err(error) => {
                    page_data.delete_error = Some(error);
                    page_data.is_deleting = false;
                }
            }
        } else {
            page_data.delete_error = Some("Not authenticated".to_string());
            page_data.is_deleting = false;
        }
        commands.entity(entity).despawn();
    }
}

pub struct ProjectSettingsPlugin;

impl Plugin for ProjectSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::ProjectSettings), setup)
           .add_systems(Update, (
               update,
               handle_save_project_task,
               handle_delete_project_task,
               handle_sync_events,
           ).run_if(in_state(AppState::ProjectSettings)))
           .add_systems(
               EguiContextPass,
               ui_system.run_if(in_state(AppState::ProjectSettings)),
           )
           .add_systems(OnExit(AppState::ProjectSettings), cleanup);
    }
}