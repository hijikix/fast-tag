use crate::ui::components::egui_common;
use crate::app::state::AppState;
use crate::auth::{AuthState, ProjectsState, fetch_projects, create_project};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiContextPass, egui};

#[derive(Resource, Default)]
pub struct ProjectsPageData {
    pub show_create_dialog: bool,
    pub new_project_name: String,
    pub new_project_description: String,
    pub create_error: Option<String>,
    pub is_creating: bool,
}

pub fn setup(
    mut commands: Commands,
    auth_state: Res<AuthState>,
    mut projects_state: ResMut<ProjectsState>,
) {
    println!("projects setup");
    
    commands.init_resource::<ProjectsPageData>();
    
    // Fetch projects if authenticated and not already fetching
    if auth_state.is_authenticated() && !projects_state.is_fetching {
        if let Some(jwt) = auth_state.get_jwt() {
            let jwt = jwt.clone();
            projects_state.start_fetching();
            
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(fetch_projects(&jwt)) {
                Ok(projects) => {
                    projects_state.set_projects(projects);
                }
                Err(error) => {
                    projects_state.set_error(error);
                }
            }
        }
    }
}

pub fn update() {
    // No update logic needed for projects page currently
}

pub fn ui_system(
    mut commands: Commands,
    mut contexts: EguiContexts,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut projects_state: ResMut<ProjectsState>,
    mut page_data: ResMut<ProjectsPageData>,
    auth_state: Res<AuthState>,
) {
    egui_common::ui_top_panel(&mut contexts, current_state, &mut next_state);

    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Projects");
            ui.add_space(10.0);
        });

        // Create new project button
        ui.horizontal(|ui| {
            if ui.button("➕ New Project").clicked() {
                page_data.show_create_dialog = true;
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh").clicked() && !projects_state.is_fetching {
                    if let Some(jwt) = auth_state.get_jwt() {
                        let jwt = jwt.clone();
                        projects_state.start_fetching();
                        
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        match rt.block_on(fetch_projects(&jwt)) {
                            Ok(projects) => {
                                projects_state.set_projects(projects);
                            }
                            Err(error) => {
                                projects_state.set_error(error);
                            }
                        }
                    }
                }
            });
        });

        ui.separator();

        // Show loading state
        if projects_state.is_fetching {
            ui.vertical_centered(|ui| {
                ui.add(egui::Spinner::new());
                ui.label("Loading projects...");
            });
            return;
        }

        // Show error state
        if let Some(error) = &projects_state.fetch_error {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
            ui.separator();
        }

        // Projects list
        egui::ScrollArea::vertical().show(ui, |ui| {
            if projects_state.projects.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No projects found");
                    ui.label("Create your first project to get started!");
                });
            } else {
                for project in &projects_state.projects {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.strong(&project.name);
                                if let Some(description) = &project.description {
                                    ui.label(description);
                                } else {
                                    ui.weak("No description");
                                }
                                ui.weak(format!("Created: {}", format_date(&project.created_at)));
                            });
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Open").clicked() {
                                    println!("Opening project: {}", project.name);
                                    // Set project ID parameter for Tasks page
                                    commands.insert_resource(crate::pages::tasks::Parameters {
                                        project_id: project.id.clone(),
                                    });
                                    next_state.set(AppState::Tasks);
                                }
                                
                                if ui.button("🔧 Settings").clicked() {
                                    // Navigate to project settings page
                                    println!("Opening settings for project: {}", project.name);
                                    // Set project ID parameter for ProjectSettings page
                                    commands.insert_resource(crate::pages::project_settings::Parameters {
                                        project_id: project.id.clone(),
                                    });
                                    next_state.set(AppState::ProjectSettings);
                                }
                            });
                        });
                    });
                    ui.add_space(5.0);
                }
            }
        });

        // Create project dialog
        show_create_project_dialog(ui, &mut page_data, &mut projects_state, &auth_state);
    });
}

fn show_create_project_dialog(
    ui: &mut egui::Ui,
    page_data: &mut ProjectsPageData,
    projects_state: &mut ProjectsState,
    auth_state: &AuthState,
) {
    if !page_data.show_create_dialog {
        return;
    }

    egui::Window::new("Create New Project")
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.vertical(|ui| {
                ui.label("Project Name:");
                ui.text_edit_singleline(&mut page_data.new_project_name);
                
                ui.add_space(10.0);
                
                ui.label("Description (optional):");
                ui.text_edit_multiline(&mut page_data.new_project_description);
                
                ui.add_space(10.0);
                
                // Show create error
                if let Some(error) = &page_data.create_error {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                }
                
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        page_data.show_create_dialog = false;
                        page_data.new_project_name.clear();
                        page_data.new_project_description.clear();
                        page_data.create_error = None;
                    }
                    
                    let can_create = !page_data.new_project_name.trim().is_empty() && !page_data.is_creating;
                    
                    if ui.add_enabled(can_create, egui::Button::new("Create")).clicked() {
                        if let Some(jwt) = auth_state.get_jwt() {
                            let jwt = jwt.clone();
                            let name = page_data.new_project_name.trim().to_string();
                            let description = if page_data.new_project_description.trim().is_empty() {
                                None
                            } else {
                                Some(page_data.new_project_description.trim())
                            };
                            
                            page_data.is_creating = true;
                            page_data.create_error = None;
                            
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            match rt.block_on(create_project(&jwt, &name, description)) {
                                Ok(project) => {
                                    projects_state.add_project(project);
                                    page_data.show_create_dialog = false;
                                    page_data.new_project_name.clear();
                                    page_data.new_project_description.clear();
                                    page_data.is_creating = false;
                                }
                                Err(error) => {
                                    page_data.create_error = Some(error);
                                    page_data.is_creating = false;
                                }
                            }
                        }
                    }
                    
                    if page_data.is_creating {
                        ui.add(egui::Spinner::new());
                    }
                });
            });
        });
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
    println!("projects cleanup");
    commands.remove_resource::<ProjectsPageData>();
}

pub struct ProjectsPlugin;

impl Plugin for ProjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Projects), setup)
           .add_systems(Update, update.run_if(in_state(AppState::Projects)))
           .add_systems(
               EguiContextPass,
               ui_system.run_if(in_state(AppState::Projects)),
           )
           .add_systems(OnExit(AppState::Projects), cleanup);
    }
}