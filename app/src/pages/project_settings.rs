use crate::ui::components::egui_common;
use crate::app::state::AppState;
use crate::auth::{AuthState, ProjectsState};
use crate::sync::{SyncState, SyncRequestEvent, SyncRequest, SyncCompletedEvent, SyncErrorEvent};
use crate::annotations::{
    AnnotationState, CreateCategoryEvent, CreateCategoryRequest,
    LoadCategoriesEvent, CategoryCreatedEvent, AnnotationErrorEvent,
};
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

#[derive(Component)]
pub struct SaveStorageConfigTask {
    pub project_id: String,
    pub storage_config: serde_json::Value,
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
    // Storage configuration fields
    pub is_editing_storage: bool,
    pub storage_provider: String,
    pub storage_s3_bucket: String,
    pub storage_s3_region: String,
    pub storage_s3_access_key: String,
    pub storage_s3_secret_key: String,
    pub storage_s3_endpoint: String,
    pub storage_azure_account_name: String,
    pub storage_azure_account_key: String,
    pub storage_azure_container_name: String,
    pub storage_gcs_bucket: String,
    pub storage_gcs_project_id: String,
    pub storage_gcs_service_account_key: String,
    pub storage_local_base_path: String,
    pub storage_save_error: Option<String>,
    pub is_saving_storage: bool,
    // Category management fields
    pub new_category_name: String,
    pub new_category_color: [f32; 3],
    pub new_category_description: String,
    pub is_creating_category: bool,
    pub category_error: Option<String>,
}

pub fn setup(
    mut commands: Commands,
    projects_state: Res<ProjectsState>,
    parameters: Option<Res<Parameters>>,
    mut annotation_state: ResMut<AnnotationState>,
    mut load_categories_events: EventWriter<LoadCategoriesEvent>,
    auth_state: Res<AuthState>,
) {
    println!("project_settings setup");
    
    let mut page_data = ProjectSettingsPageData {
        new_category_color: [1.0, 0.0, 0.0], // Default to red
        ..Default::default()
    };
    
    // Initialize with project from parameters if available, otherwise use first project
    let selected_project_id = if let Some(params) = parameters {
        if let Some(project) = projects_state.projects.iter().find(|p| p.id == params.project_id) {
            page_data.selected_project_id = Some(project.id.clone());
            page_data.project_name = project.name.clone();
            page_data.project_description = project.description.clone().unwrap_or_default();
            
            // Initialize storage configuration from project
            if let Some(storage_config) = &project.storage_config {
                parse_storage_config(&mut page_data, storage_config);
            }
            
            Some(project.id.clone())
        } else {
            None
        }
    } else if let Some(project) = projects_state.projects.first() {
        page_data.selected_project_id = Some(project.id.clone());
        page_data.project_name = project.name.clone();
        page_data.project_description = project.description.clone().unwrap_or_default();
        
        // Initialize storage configuration from project
        if let Some(storage_config) = &project.storage_config {
            parse_storage_config(&mut page_data, storage_config);
        }
        
        Some(project.id.clone())
    } else {
        None
    };
    
    // Load categories for the selected project
    if let (Some(project_id_str), Some(token)) = (selected_project_id, auth_state.get_jwt()) {
        if let Ok(project_uuid) = Uuid::parse_str(&project_id_str) {
            annotation_state.current_project_id = Some(project_uuid);
            // Load categories for this project
            load_categories_events.write(LoadCategoriesEvent {
                project_id: project_uuid,
                token: token.clone(),
            });
        }
    }
    
    commands.insert_resource(page_data);
}

fn build_storage_config(page_data: &ProjectSettingsPageData) -> Option<serde_json::Value> {
    use serde_json::json;
    
    match page_data.storage_provider.as_str() {
        "s3" => {
            if page_data.storage_s3_bucket.is_empty() || 
               page_data.storage_s3_region.is_empty() || 
               page_data.storage_s3_access_key.is_empty() || 
               page_data.storage_s3_secret_key.is_empty() {
                return None;
            }
            
            let mut config = json!({
                "type": "s3",
                "bucket": page_data.storage_s3_bucket.trim(),
                "region": page_data.storage_s3_region.trim(),
                "access_key": page_data.storage_s3_access_key.trim(),
                "secret_key": page_data.storage_s3_secret_key.trim(),
            });
            
            if !page_data.storage_s3_endpoint.is_empty() {
                config["endpoint"] = json!(page_data.storage_s3_endpoint.trim());
            }
            
            Some(config)
        }
        "azure" => {
            if page_data.storage_azure_account_name.is_empty() || 
               page_data.storage_azure_account_key.is_empty() || 
               page_data.storage_azure_container_name.is_empty() {
                return None;
            }
            
            Some(json!({
                "type": "azure",
                "account_name": page_data.storage_azure_account_name.trim(),
                "account_key": page_data.storage_azure_account_key.trim(),
                "container_name": page_data.storage_azure_container_name.trim(),
            }))
        }
        "gcs" => {
            if page_data.storage_gcs_bucket.is_empty() || 
               page_data.storage_gcs_project_id.is_empty() || 
               page_data.storage_gcs_service_account_key.is_empty() {
                return None;
            }
            
            Some(json!({
                "type": "gcs",
                "bucket": page_data.storage_gcs_bucket.trim(),
                "project_id": page_data.storage_gcs_project_id.trim(),
                "service_account_key": page_data.storage_gcs_service_account_key.trim(),
            }))
        }
        "local" => {
            if page_data.storage_local_base_path.is_empty() {
                return None;
            }
            
            Some(json!({
                "type": "local",
                "base_path": page_data.storage_local_base_path.trim(),
            }))
        }
        _ => None,
    }
}

fn parse_storage_config(page_data: &mut ProjectSettingsPageData, storage_config: &serde_json::Value) {
    if let Some(provider_type) = storage_config.get("type").and_then(|v| v.as_str()) {
        page_data.storage_provider = provider_type.to_string();
        
        match provider_type {
            "s3" => {
                if let Some(bucket) = storage_config.get("bucket").and_then(|v| v.as_str()) {
                    page_data.storage_s3_bucket = bucket.to_string();
                }
                if let Some(region) = storage_config.get("region").and_then(|v| v.as_str()) {
                    page_data.storage_s3_region = region.to_string();
                }
                if let Some(access_key) = storage_config.get("access_key").and_then(|v| v.as_str()) {
                    page_data.storage_s3_access_key = access_key.to_string();
                }
                if let Some(secret_key) = storage_config.get("secret_key").and_then(|v| v.as_str()) {
                    page_data.storage_s3_secret_key = secret_key.to_string();
                }
                if let Some(endpoint) = storage_config.get("endpoint").and_then(|v| v.as_str()) {
                    page_data.storage_s3_endpoint = endpoint.to_string();
                }
            }
            "azure" => {
                if let Some(account_name) = storage_config.get("account_name").and_then(|v| v.as_str()) {
                    page_data.storage_azure_account_name = account_name.to_string();
                }
                if let Some(account_key) = storage_config.get("account_key").and_then(|v| v.as_str()) {
                    page_data.storage_azure_account_key = account_key.to_string();
                }
                if let Some(container_name) = storage_config.get("container_name").and_then(|v| v.as_str()) {
                    page_data.storage_azure_container_name = container_name.to_string();
                }
            }
            "gcs" => {
                if let Some(bucket) = storage_config.get("bucket").and_then(|v| v.as_str()) {
                    page_data.storage_gcs_bucket = bucket.to_string();
                }
                if let Some(project_id) = storage_config.get("project_id").and_then(|v| v.as_str()) {
                    page_data.storage_gcs_project_id = project_id.to_string();
                }
                if let Some(service_account_key) = storage_config.get("service_account_key").and_then(|v| v.as_str()) {
                    page_data.storage_gcs_service_account_key = service_account_key.to_string();
                }
            }
            "local" => {
                if let Some(base_path) = storage_config.get("base_path").and_then(|v| v.as_str()) {
                    page_data.storage_local_base_path = base_path.to_string();
                }
            }
            _ => {}
        }
    }
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
    annotation_state: Res<AnnotationState>,
    mut sync_request_events: EventWriter<SyncRequestEvent>,
    mut create_category_events: EventWriter<CreateCategoryEvent>,
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
                
                // Storage Configuration section
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong("Storage Configuration");
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if page_data.is_editing_storage {
                                    let can_save = !page_data.is_saving_storage;
                                    
                                    if ui.add_enabled(can_save, egui::Button::new("💾 Save")).clicked() {
                                        if let Some(storage_config) = build_storage_config(&page_data) {
                                            page_data.is_saving_storage = true;
                                            page_data.storage_save_error = None;
                                            
                                            // Spawn task to save storage config
                                            commands.spawn(SaveStorageConfigTask {
                                                project_id: project_id.clone(),
                                                storage_config,
                                            });
                                        } else {
                                            page_data.storage_save_error = Some("Invalid storage configuration".to_string());
                                        }
                                    }
                                    
                                    if ui.button("❌ Cancel").clicked() {
                                        // Reset fields from project's current storage config
                                        if let Some(storage_config) = &project.storage_config {
                                            parse_storage_config(&mut page_data, storage_config);
                                        } else {
                                            // Clear all fields if no config exists
                                            page_data.storage_provider = String::new();
                                            page_data.storage_s3_bucket = String::new();
                                            page_data.storage_s3_region = String::new();
                                            page_data.storage_s3_access_key = String::new();
                                            page_data.storage_s3_secret_key = String::new();
                                            page_data.storage_s3_endpoint = String::new();
                                            page_data.storage_azure_account_name = String::new();
                                            page_data.storage_azure_account_key = String::new();
                                            page_data.storage_azure_container_name = String::new();
                                            page_data.storage_gcs_bucket = String::new();
                                            page_data.storage_gcs_project_id = String::new();
                                            page_data.storage_gcs_service_account_key = String::new();
                                            page_data.storage_local_base_path = String::new();
                                        }
                                        page_data.is_editing_storage = false;
                                        page_data.storage_save_error = None;
                                    }
                                    
                                    if page_data.is_saving_storage {
                                        ui.add(egui::Spinner::new());
                                    }
                                } else if ui.button("Configure").clicked() {
                                    page_data.is_editing_storage = true;
                                }
                            });
                        });
                        
                        ui.separator();
                        ui.add_space(10.0);
                        
                        if page_data.is_editing_storage {
                            // Provider selection
                            ui.horizontal(|ui| {
                                ui.label("Provider:");
                                egui::ComboBox::from_label("")
                                    .selected_text(if page_data.storage_provider.is_empty() { 
                                        "Select provider" 
                                    } else { 
                                        &page_data.storage_provider 
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut page_data.storage_provider, "s3".to_string(), "Amazon S3");
                                        ui.selectable_value(&mut page_data.storage_provider, "azure".to_string(), "Azure Blob Storage");
                                        ui.selectable_value(&mut page_data.storage_provider, "gcs".to_string(), "Google Cloud Storage");
                                        ui.selectable_value(&mut page_data.storage_provider, "local".to_string(), "Local Storage");
                                    });
                            });
                            
                            ui.add_space(10.0);
                            
                            // Provider-specific fields
                            match page_data.storage_provider.as_str() {
                                "s3" => {
                                    ui.horizontal(|ui| {
                                        ui.label("Bucket:");
                                        ui.text_edit_singleline(&mut page_data.storage_s3_bucket);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Region:");
                                        ui.text_edit_singleline(&mut page_data.storage_s3_region);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Access Key:");
                                        ui.text_edit_singleline(&mut page_data.storage_s3_access_key);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Secret Key:");
                                        ui.add(egui::TextEdit::singleline(&mut page_data.storage_s3_secret_key).password(true));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Endpoint (optional):");
                                        ui.text_edit_singleline(&mut page_data.storage_s3_endpoint);
                                    });
                                }
                                "azure" => {
                                    ui.horizontal(|ui| {
                                        ui.label("Account Name:");
                                        ui.text_edit_singleline(&mut page_data.storage_azure_account_name);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Account Key:");
                                        ui.add(egui::TextEdit::singleline(&mut page_data.storage_azure_account_key).password(true));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Container Name:");
                                        ui.text_edit_singleline(&mut page_data.storage_azure_container_name);
                                    });
                                }
                                "gcs" => {
                                    ui.horizontal(|ui| {
                                        ui.label("Bucket:");
                                        ui.text_edit_singleline(&mut page_data.storage_gcs_bucket);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Project ID:");
                                        ui.text_edit_singleline(&mut page_data.storage_gcs_project_id);
                                    });
                                    ui.vertical(|ui| {
                                        ui.label("Service Account Key (JSON):");
                                        ui.text_edit_multiline(&mut page_data.storage_gcs_service_account_key);
                                    });
                                }
                                "local" => {
                                    ui.horizontal(|ui| {
                                        ui.label("Base Path:");
                                        ui.text_edit_singleline(&mut page_data.storage_local_base_path);
                                    });
                                }
                                _ => {
                                    ui.label("Please select a storage provider.");
                                }
                            }
                        } else {
                            // Show current configuration (read-only)
                            if let Some(storage_config) = &project.storage_config {
                                if let Some(provider_type) = storage_config.get("type").and_then(|v| v.as_str()) {
                                    ui.horizontal(|ui| {
                                        ui.label("Provider:");
                                        ui.label(match provider_type {
                                            "s3" => "Amazon S3",
                                            "azure" => "Azure Blob Storage",
                                            "gcs" => "Google Cloud Storage",
                                            "local" => "Local Storage",
                                            _ => provider_type,
                                        });
                                    });
                                    
                                    match provider_type {
                                        "s3" => {
                                            if let Some(bucket) = storage_config.get("bucket").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Bucket:");
                                                    ui.label(bucket);
                                                });
                                            }
                                            if let Some(region) = storage_config.get("region").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Region:");
                                                    ui.label(region);
                                                });
                                            }
                                        }
                                        "azure" => {
                                            if let Some(account_name) = storage_config.get("account_name").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Account Name:");
                                                    ui.label(account_name);
                                                });
                                            }
                                            if let Some(container_name) = storage_config.get("container_name").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Container:");
                                                    ui.label(container_name);
                                                });
                                            }
                                        }
                                        "gcs" => {
                                            if let Some(bucket) = storage_config.get("bucket").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Bucket:");
                                                    ui.label(bucket);
                                                });
                                            }
                                            if let Some(project_id) = storage_config.get("project_id").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Project ID:");
                                                    ui.label(project_id);
                                                });
                                            }
                                        }
                                        "local" => {
                                            if let Some(base_path) = storage_config.get("base_path").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Base Path:");
                                                    ui.label(base_path);
                                                });
                                            }
                                        }
                                        _ => {}
                                    }
                                } else {
                                    ui.label("No storage configuration set.");
                                }
                            } else {
                                ui.label("No storage configuration set.");
                            }
                        }
                        
                        // Show save error
                        if let Some(error) = &page_data.storage_save_error {
                            ui.add_space(10.0);
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                        }
                    });
                });
                
                ui.add_space(20.0);
                
                // Category Management section
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.strong("Annotation Categories");
                        ui.separator();
                        ui.add_space(10.0);
                        
                        // Show existing categories
                        ui.label("Existing Categories:");
                        ui.add_space(5.0);
                        
                        if annotation_state.categories.is_empty() {
                            ui.colored_label(egui::Color32::GRAY, "No categories created yet.");
                        } else {
                            for category in &annotation_state.categories {
                                ui.horizontal(|ui| {
                                    // Color indicator
                                    if let Some(color) = &category.color {
                                        if let Ok(hex) = u32::from_str_radix(&color[1..], 16) {
                                            let r = ((hex >> 16) & 0xFF) as u8;
                                            let g = ((hex >> 8) & 0xFF) as u8;
                                            let b = (hex & 0xFF) as u8;
                                            let egui_color = egui::Color32::from_rgb(r, g, b);
                                            ui.painter().rect_filled(
                                                egui::Rect::from_min_size(
                                                    ui.cursor().min,
                                                    egui::Vec2::new(12.0, 12.0)
                                                ),
                                                2.0,
                                                egui_color,
                                            );
                                            ui.add_space(16.0);
                                        }
                                    }
                                    
                                    ui.label(&category.name);
                                    
                                    if let Some(description) = &category.description {
                                        ui.label(format!("- {}", description));
                                    }
                                });
                            }
                        }
                        
                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(10.0);
                        
                        // Create new category form
                        ui.label("Create New Category:");
                        ui.add_space(5.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut page_data.new_category_name);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Color:");
                            egui::color_picker::color_edit_button_rgb(ui, &mut page_data.new_category_color);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Description:");
                            ui.text_edit_singleline(&mut page_data.new_category_description);
                        });
                        
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            let can_create = !page_data.new_category_name.trim().is_empty() && 
                                           !page_data.is_creating_category;
                            
                            if ui.add_enabled(can_create, egui::Button::new("➕ Create Category")).clicked() {
                                if let Some(token) = auth_state.get_jwt() {
                                    if let Some(project_id_str) = &page_data.selected_project_id {
                                        if let Ok(project_uuid) = Uuid::parse_str(project_id_str) {
                                            page_data.is_creating_category = true;
                                            page_data.category_error = None;
                                            
                                            // Convert RGB color to hex
                                            let hex_color = format!(
                                                "#{:02x}{:02x}{:02x}",
                                                (page_data.new_category_color[0] * 255.0) as u8,
                                                (page_data.new_category_color[1] * 255.0) as u8,
                                                (page_data.new_category_color[2] * 255.0) as u8
                                            );
                                            
                                            let request = CreateCategoryRequest {
                                                name: page_data.new_category_name.clone(),
                                                supercategory: None,
                                                color: Some(hex_color),
                                                description: if page_data.new_category_description.trim().is_empty() {
                                                    None
                                                } else {
                                                    Some(page_data.new_category_description.clone())
                                                },
                                                coco_id: None,
                                            };
                                            
                                            create_category_events.write(CreateCategoryEvent {
                                                project_id: project_uuid,
                                                request,
                                                token: token.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                            
                            if page_data.is_creating_category {
                                ui.add(egui::Spinner::new());
                                ui.label("Creating...");
                            }
                        });
                        
                        // Show category error
                        if let Some(error) = &page_data.category_error {
                            ui.add_space(10.0);
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
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

pub fn handle_category_events(
    mut page_data: ResMut<ProjectSettingsPageData>,
    mut category_created_events: EventReader<CategoryCreatedEvent>,
    mut annotation_error_events: EventReader<AnnotationErrorEvent>,
) {
    for event in category_created_events.read() {
        info!("Category created: {}", event.category.name);
        page_data.is_creating_category = false;
        page_data.category_error = None;
        // Reset form
        page_data.new_category_name.clear();
        page_data.new_category_color = [1.0, 0.0, 0.0];
        page_data.new_category_description.clear();
    }
    
    for event in annotation_error_events.read() {
        error!("Category creation error: {}", event.error);
        page_data.is_creating_category = false;
        page_data.category_error = Some(event.error.clone());
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

pub fn handle_save_storage_config_task(
    mut commands: Commands,
    mut page_data: ResMut<ProjectSettingsPageData>,
    mut projects_state: ResMut<ProjectsState>,
    auth_state: Res<AuthState>,
    mut save_tasks: Query<(Entity, &SaveStorageConfigTask)>,
) {
    for (entity, task) in save_tasks.iter_mut() {
        if let Some(jwt) = auth_state.get_jwt() {
            let jwt = jwt.clone();
            let project_id = task.project_id.clone();
            let storage_config = task.storage_config.clone();
            
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(crate::auth::update_project_storage_config(&jwt, &project_id, storage_config)) {
                Ok(updated_project) => {
                    // Update the project in projects_state
                    if let Some(project) = projects_state.projects.iter_mut().find(|p| p.id == updated_project.id) {
                        *project = updated_project.clone();
                        
                        // Update page data to reflect the changes
                        if Some(&project.id) == page_data.selected_project_id.as_ref() {
                            if let Some(storage_config) = &updated_project.storage_config {
                                parse_storage_config(&mut page_data, storage_config);
                            }
                        }
                    }
                    
                    page_data.is_saving_storage = false;
                    page_data.is_editing_storage = false;
                    page_data.storage_save_error = None;
                }
                Err(error) => {
                    page_data.storage_save_error = Some(error);
                    page_data.is_saving_storage = false;
                }
            }
        } else {
            page_data.storage_save_error = Some("Not authenticated".to_string());
            page_data.is_saving_storage = false;
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
               handle_save_storage_config_task,
               handle_sync_events,
               handle_category_events,
           ).run_if(in_state(AppState::ProjectSettings)))
           .add_systems(
               EguiContextPass,
               ui_system.run_if(in_state(AppState::ProjectSettings)),
           )
           .add_systems(OnExit(AppState::ProjectSettings), cleanup);
    }
}