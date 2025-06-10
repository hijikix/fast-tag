use bevy::prelude::*;
use serde::{Deserialize, Serialize, Deserializer};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

pub mod client;

pub struct AnnotationPlugin;

impl Plugin for AnnotationPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = channel::<AnnotationResult>();
        
        app
            .init_resource::<AnnotationState>()
            .insert_resource(AnnotationChannelSender(Mutex::new(tx)))
            .insert_resource(AnnotationChannelReceiver(Mutex::new(rx)))
            .add_event::<SaveAnnotationEvent>()
            .add_event::<LoadAnnotationEvent>()
            .add_event::<LoadCategoriesEvent>()
            .add_event::<CreateCategoryEvent>()
            .add_event::<AnnotationSavedEvent>()
            .add_event::<AnnotationLoadedEvent>()
            .add_event::<CategoryCreatedEvent>()
            .add_event::<AnnotationErrorEvent>()
            .add_systems(Update, (
                handle_annotation_requests,
                process_annotation_results,
            ));
    }
}

#[derive(Resource, Default)]
pub struct AnnotationState {
    pub is_saving: bool,
    pub categories: Vec<AnnotationCategory>,
    pub current_task_id: Option<Uuid>,
    pub current_project_id: Option<Uuid>,
}

#[derive(Resource)]
pub struct AnnotationChannelSender(Mutex<Sender<AnnotationResult>>);

#[derive(Resource)]
pub struct AnnotationChannelReceiver(Mutex<Receiver<AnnotationResult>>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnotationCategory {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub supercategory: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub coco_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Annotation {
    pub id: Uuid,
    pub task_id: Uuid,
    pub metadata: serde_json::Value,
    pub annotated_by: Option<Uuid>,
    pub annotated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageAnnotation {
    #[serde(rename = "id")]
    pub image_id: Uuid,
    pub annotation_id: Uuid,
    pub category_id: Option<Uuid>,
    pub bbox: Vec<f64>,
    pub area: Option<f64>,
    pub iscrowd: bool,
    pub image_metadata: serde_json::Value,
    #[serde(rename = "created_at")]
    pub image_created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    pub image_updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnnotationWithCategory {
    // Annotation fields
    pub id: Uuid,
    pub task_id: Uuid,
    pub metadata: serde_json::Value,
    pub annotated_by: Option<Uuid>,
    pub annotated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Image annotation fields
    pub image_id: Uuid,
    pub annotation_id: Uuid,
    pub category_id: Option<Uuid>,
    pub bbox: Vec<f64>,
    pub area: Option<f64>,
    pub iscrowd: bool,
    pub image_metadata: serde_json::Value,
    // Category fields
    pub category_name: String,
    pub category_color: Option<String>,
}

impl<'de> Deserialize<'de> for AnnotationWithCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct AnnotationWithCategoryVisitor;

        impl<'de> Visitor<'de> for AnnotationWithCategoryVisitor {
            type Value = AnnotationWithCategory;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct AnnotationWithCategory")
            }

            fn visit_map<V>(self, mut map: V) -> Result<AnnotationWithCategory, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id: Option<Uuid> = None;
                let mut image_id: Option<Uuid> = None;
                let mut task_id = None;
                let mut metadata = None;
                let mut annotated_by = None;
                let mut annotated_at = None;
                let mut created_at = None;
                let mut updated_at = None;
                let mut annotation_id = None;
                let mut category_id = None;
                let mut bbox = None;
                let mut area = None;
                let mut iscrowd = None;
                let mut image_metadata = None;
                let mut category_name = None;
                let mut category_color = None;
                
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "id" => {
                            if id.is_none() {
                                id = Some(map.next_value()?);
                            } else {
                                // Second ID field is the image ID
                                image_id = Some(map.next_value()?);
                            }
                        }
                        "task_id" => task_id = Some(map.next_value()?),
                        "metadata" => metadata = Some(map.next_value()?),
                        "annotated_by" => annotated_by = Some(map.next_value()?),
                        "annotated_at" => annotated_at = Some(map.next_value()?),
                        "created_at" => {
                            if created_at.is_none() {
                                created_at = Some(map.next_value()?);
                            } else {
                                // Skip second created_at
                                let _: DateTime<Utc> = map.next_value()?;
                            }
                        }
                        "updated_at" => {
                            if updated_at.is_none() {
                                updated_at = Some(map.next_value()?);
                            } else {
                                // Skip second updated_at
                                let _: DateTime<Utc> = map.next_value()?;
                            }
                        }
                        "annotation_id" => annotation_id = Some(map.next_value()?),
                        "category_id" => category_id = Some(map.next_value()?),
                        "bbox" => bbox = Some(map.next_value()?),
                        "area" => area = Some(map.next_value()?),
                        "iscrowd" => iscrowd = Some(map.next_value()?),
                        "image_metadata" => image_metadata = Some(map.next_value()?),
                        "category_name" => category_name = Some(map.next_value()?),
                        "category_color" => category_color = Some(map.next_value()?),
                        _ => {
                            // Skip unknown fields
                            let _ = map.next_value::<serde_json::Value>()?;
                        }
                    }
                }

                Ok(AnnotationWithCategory {
                    id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                    task_id: task_id.ok_or_else(|| de::Error::missing_field("task_id"))?,
                    metadata: metadata.ok_or_else(|| de::Error::missing_field("metadata"))?,
                    annotated_by,
                    annotated_at: annotated_at.ok_or_else(|| de::Error::missing_field("annotated_at"))?,
                    created_at: created_at.ok_or_else(|| de::Error::missing_field("created_at"))?,
                    updated_at: updated_at.ok_or_else(|| de::Error::missing_field("updated_at"))?,
                    image_id: image_id.ok_or_else(|| de::Error::missing_field("image_id"))?,
                    annotation_id: annotation_id.ok_or_else(|| de::Error::missing_field("annotation_id"))?,
                    category_id,
                    bbox: bbox.ok_or_else(|| de::Error::missing_field("bbox"))?,
                    area,
                    iscrowd: iscrowd.ok_or_else(|| de::Error::missing_field("iscrowd"))?,
                    image_metadata: image_metadata.ok_or_else(|| de::Error::missing_field("image_metadata"))?,
                    category_name: category_name.ok_or_else(|| de::Error::missing_field("category_name"))?,
                    category_color,
                })
            }
        }

        deserializer.deserialize_map(AnnotationWithCategoryVisitor)
    }
}

impl AnnotationWithCategory {}

#[derive(Debug, Serialize, Clone)]
pub struct CreateAnnotationRequest {
    pub category_id: Uuid,
    pub bbox: Vec<f64>,
    pub area: Option<f64>,
    pub iscrowd: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub supercategory: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub coco_id: Option<i32>,
}

enum AnnotationResult {
    AnnotationSaved { annotations: Vec<AnnotationWithCategory> },
    AnnotationsLoaded { annotations: Vec<AnnotationWithCategory> },
    CategoriesLoaded { categories: Vec<AnnotationCategory> },
    CategoryCreated { category: AnnotationCategory },
    Error { error: String },
}

#[derive(Event)]
pub struct SaveAnnotationEvent {
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub annotations: Vec<CreateAnnotationRequest>,
    pub token: String,
}

#[derive(Event)]
pub struct LoadAnnotationEvent {
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub token: String,
}

#[derive(Event)]
pub struct LoadCategoriesEvent {
    pub project_id: Uuid,
    pub token: String,
}

#[derive(Event)]
pub struct CreateCategoryEvent {
    pub project_id: Uuid,
    pub request: CreateCategoryRequest,
    pub token: String,
}

#[derive(Event)]
pub struct AnnotationSavedEvent {
    pub annotations: Vec<AnnotationWithCategory>,
}

#[derive(Event)]
pub struct AnnotationLoadedEvent {
    pub annotations: Vec<AnnotationWithCategory>,
}

#[derive(Event)]
pub struct CategoryCreatedEvent {
    pub category: AnnotationCategory,
}

#[derive(Event)]
pub struct AnnotationErrorEvent {
    pub error: String,
}

fn handle_annotation_requests(
    mut save_events: EventReader<SaveAnnotationEvent>,
    mut load_events: EventReader<LoadAnnotationEvent>,
    mut load_categories_events: EventReader<LoadCategoriesEvent>,
    mut create_category_events: EventReader<CreateCategoryEvent>,
    sender: Res<AnnotationChannelSender>,
    mut annotation_state: ResMut<AnnotationState>,
) {
    for save_event in save_events.read() {
        if annotation_state.is_saving {
            if let Ok(tx) = sender.0.lock() {
                let _ = tx.send(AnnotationResult::Error {
                    error: "Save already in progress".to_string(),
                });
            }
            continue;
        }
        
        annotation_state.is_saving = true;
        
        let project_id = save_event.project_id;
        let task_id = save_event.task_id;
        let annotations = save_event.annotations.clone();
        let token = save_event.token.clone();
        let api_base_url = std::env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        
        if let Ok(tx) = sender.0.lock() {
            let tx = tx.clone();
            
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    let result = client::save_annotations(project_id, task_id, annotations, token, api_base_url).await;
                    match result {
                        Ok(saved_annotations) => {
                            let _ = tx.send(AnnotationResult::AnnotationSaved { annotations: saved_annotations });
                        }
                        Err(error) => {
                            let _ = tx.send(AnnotationResult::Error { error });
                        }
                    }
                });
            });
        }
    }

    for load_event in load_events.read() {
        let project_id = load_event.project_id;
        let task_id = load_event.task_id;
        let token = load_event.token.clone();
        let api_base_url = std::env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        
        if let Ok(tx) = sender.0.lock() {
            let tx = tx.clone();
            
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    // Load both annotations and categories
                    let annotations_result = client::load_annotations(project_id, task_id, token.clone(), api_base_url.clone()).await;
                    let categories_result = client::load_categories(project_id, token, api_base_url).await;
                    
                    match (annotations_result, categories_result) {
                        (Ok(annotations), Ok(categories)) => {
                            let _ = tx.send(AnnotationResult::AnnotationsLoaded { annotations });
                            let _ = tx.send(AnnotationResult::CategoriesLoaded { categories });
                        }
                        (Err(error), _) | (_, Err(error)) => {
                            let _ = tx.send(AnnotationResult::Error { error });
                        }
                    }
                });
            });
        }
    }

    for load_categories_event in load_categories_events.read() {
        let project_id = load_categories_event.project_id;
        let token = load_categories_event.token.clone();
        let api_base_url = std::env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        
        if let Ok(tx) = sender.0.lock() {
            let tx = tx.clone();
            
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    let result = client::load_categories(project_id, token, api_base_url).await;
                    match result {
                        Ok(categories) => {
                            let _ = tx.send(AnnotationResult::CategoriesLoaded { categories });
                        }
                        Err(error) => {
                            let _ = tx.send(AnnotationResult::Error { error });
                        }
                    }
                });
            });
        }
    }

    for create_event in create_category_events.read() {
        let project_id = create_event.project_id;
        let request = create_event.request.clone();
        let token = create_event.token.clone();
        let api_base_url = std::env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        
        if let Ok(tx) = sender.0.lock() {
            let tx = tx.clone();
            
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    let result = client::create_category(project_id, request, token, api_base_url).await;
                    match result {
                        Ok(category) => {
                            let _ = tx.send(AnnotationResult::CategoryCreated { category });
                        }
                        Err(error) => {
                            let _ = tx.send(AnnotationResult::Error { error });
                        }
                    }
                });
            });
        }
    }
}

fn process_annotation_results(
    receiver: Res<AnnotationChannelReceiver>,
    mut annotation_state: ResMut<AnnotationState>,
    mut saved_events: EventWriter<AnnotationSavedEvent>,
    mut loaded_events: EventWriter<AnnotationLoadedEvent>,
    mut category_created_events: EventWriter<CategoryCreatedEvent>,
    mut error_events: EventWriter<AnnotationErrorEvent>,
) {
    if let Ok(rx) = receiver.0.lock() {
        while let Ok(result) = rx.try_recv() {
            match result {
                AnnotationResult::AnnotationSaved { annotations } => {
                    annotation_state.is_saving = false;
                    saved_events.write(AnnotationSavedEvent { annotations });
                }
                AnnotationResult::AnnotationsLoaded { annotations } => {
                    loaded_events.write(AnnotationLoadedEvent { annotations });
                }
                AnnotationResult::CategoriesLoaded { categories } => {
                    annotation_state.categories = categories;
                }
                AnnotationResult::CategoryCreated { category } => {
                    annotation_state.categories.push(category.clone());
                    category_created_events.write(CategoryCreatedEvent { category });
                }
                AnnotationResult::Error { error } => {
                    annotation_state.is_saving = false;
                    error_events.write(AnnotationErrorEvent { error });
                }
            }
        }
    }
}