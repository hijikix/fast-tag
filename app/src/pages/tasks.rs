use crate::ui::components::egui_common;
use crate::app::state::AppState;
use crate::auth::{AuthState, Task, fetch_tasks};
use bevy::prelude::*;
use bevy::ui::Interaction;
use bevy_egui::{EguiContexts, EguiContextPass, egui};

use super::detail;

#[derive(Resource, Default)]
pub struct Parameters {
    pub project_id: String,
}

#[derive(Resource, Default)]
pub struct TasksPageData {
    pub show_create_dialog: bool,
    #[allow(dead_code)]
    pub new_task_name: String,
    #[allow(dead_code)]
    pub new_task_resource_url: String,
    #[allow(dead_code)]
    pub create_error: Option<String>,
    #[allow(dead_code)]
    pub is_creating: bool,
}

#[derive(Resource, Default)]
pub struct TasksState {
    pub tasks: Vec<Task>,
    pub fetch_error: Option<String>,
    pub is_fetching: bool,
}

impl TasksState {
    pub fn set_tasks(&mut self, tasks: Vec<Task>) {
        self.tasks = tasks;
        self.fetch_error = None;
        self.is_fetching = false;
    }
    
    pub fn set_error(&mut self, error: String) {
        self.fetch_error = Some(error);
        self.is_fetching = false;
    }
    
    pub fn start_fetching(&mut self) {
        self.is_fetching = true;
        self.fetch_error = None;
    }
    
    #[allow(dead_code)]
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }
    
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.tasks.clear();
        self.fetch_error = None;
        self.is_fetching = false;
    }
}


pub fn setup(
    mut commands: Commands,
    auth_state: Res<AuthState>,
    mut tasks_state: ResMut<TasksState>,
    parameters: Option<Res<Parameters>>,
) {
    println!("tasks setup");
    
    commands.init_resource::<TasksPageData>();
    
    // Fetch tasks if authenticated and we have a project ID
    if let Some(params) = parameters {
        if auth_state.is_authenticated() && !tasks_state.is_fetching {
            if let Some(jwt) = auth_state.get_jwt() {
                let jwt = jwt.clone();
                let project_id = params.project_id.clone();
                tasks_state.start_fetching();
                
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(fetch_tasks(&jwt, &project_id)) {
                    Ok(tasks) => {
                        tasks_state.set_tasks(tasks);
                    }
                    Err(error) => {
                        tasks_state.set_error(error);
                    }
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn update(
    _params: ResMut<detail::Parameters>,
    _next_state: ResMut<NextState<AppState>>,
    _interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
}


pub fn ui_system(
    mut commands: Commands,
    mut contexts: EguiContexts,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut tasks_state: ResMut<TasksState>,
    mut page_data: ResMut<TasksPageData>,
    auth_state: Res<AuthState>,
    parameters: Option<Res<Parameters>>,
) {
    egui_common::ui_top_panel(&mut contexts, current_state, &mut next_state);

    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Tasks");
            ui.add_space(10.0);
        });

        // Create new task button and refresh
        ui.horizontal(|ui| {
            if ui.button("➕ New Task").clicked() {
                page_data.show_create_dialog = true;
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh").clicked() && !tasks_state.is_fetching {
                    if let (Some(jwt), Some(params)) = (auth_state.get_jwt(), &parameters) {
                        let jwt = jwt.clone();
                        let project_id = params.project_id.clone();
                        tasks_state.start_fetching();
                        
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        match rt.block_on(fetch_tasks(&jwt, &project_id)) {
                            Ok(tasks) => {
                                tasks_state.set_tasks(tasks);
                            }
                            Err(error) => {
                                tasks_state.set_error(error);
                            }
                        }
                    }
                }
                
                if ui.button("← Back to Projects").clicked() {
                    next_state.set(AppState::Projects);
                }
            });
        });

        ui.separator();

        // Show loading state
        if tasks_state.is_fetching {
            ui.vertical_centered(|ui| {
                ui.add(egui::Spinner::new());
                ui.label("Loading tasks...");
            });
            return;
        }

        // Show error state
        if let Some(error) = &tasks_state.fetch_error {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
            ui.separator();
        }

        // Tasks list
        egui::ScrollArea::vertical().show(ui, |ui| {
            if tasks_state.tasks.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No tasks found");
                    ui.label("Create your first task to get started!");
                });
            } else {
                for task in &tasks_state.tasks {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.strong(&task.name);
                                ui.label(format!("Status: {}", format_status(&task.status)));
                                if let Some(url) = &task.resource_url {
                                    ui.weak(format!("Resource: {}", url));
                                }
                                ui.weak(format!("Created: {}", format_date(&task.created_at)));
                            });
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Open").clicked() && task.resource_url.is_some() {
                                    println!("Opening task: {} (ID: {})", task.name, task.id);
                                    // Set task parameters for Detail page
                                    commands.insert_resource(detail::Parameters {
                                        url: String::new(), // Will be fetched from task
                                        task_id: Some(task.id.clone()),
                                        project_id: parameters.as_ref().map(|p| p.project_id.clone()),
                                    });
                                    next_state.set(AppState::Detail);
                                }
                            });
                        });
                    });
                    ui.add_space(5.0);
                }
            }
        });

        // Create task dialog would go here if needed
        // show_create_task_dialog(ui, &mut page_data, &mut tasks_state, &auth_state, &parameters);
    });
}

pub fn cleanup(mut commands: Commands) {
    println!("tasks cleanup");
    commands.remove_resource::<TasksPageData>();
}

fn format_status(status: &str) -> String {
    match status {
        "pending" => "📋 Pending".to_string(),
        "in_progress" => "🔄 In Progress".to_string(),
        "completed" => "✅ Completed".to_string(),
        "cancelled" => "❌ Cancelled".to_string(),
        _ => status.to_string(),
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

pub struct TasksPlugin;

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TasksState>()
           .add_systems(OnEnter(AppState::Tasks), setup)
           .add_systems(Update, update.run_if(in_state(AppState::Tasks)))
           .add_systems(
               EguiContextPass,
               ui_system.run_if(in_state(AppState::Tasks)),
           )
           .add_systems(OnExit(AppState::Tasks), cleanup);
    }
}
