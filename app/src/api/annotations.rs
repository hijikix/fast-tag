use super::{ApiClient, ApiResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnotationWithCategory {
    // When flattened, ImageAnnotation fields overwrite Annotation fields with same name
    // So 'id' will be the ImageAnnotation.id, not Annotation.id
    pub id: Uuid, // This is actually ImageAnnotation.id
    pub task_id: Uuid, // From Annotation
    pub metadata: serde_json::Value, // From Annotation
    pub annotated_by: Option<Uuid>, // From Annotation
    pub annotated_at: DateTime<Utc>, // From Annotation
    pub annotation_id: Uuid, // From ImageAnnotation
    pub category_id: Option<Uuid>, // From ImageAnnotation
    pub bbox: Vec<f64>, // From ImageAnnotation
    pub area: Option<f64>, // From ImageAnnotation
    pub iscrowd: bool, // From ImageAnnotation
    pub image_metadata: serde_json::Value, // From ImageAnnotation
    pub created_at: DateTime<Utc>, // This is actually ImageAnnotation.created_at
    pub updated_at: DateTime<Utc>, // This is actually ImageAnnotation.updated_at
    // Category fields
    pub category_name: String,
    pub category_color: Option<String>,
}


#[derive(Debug, Serialize, Clone)]
pub struct CreateAnnotationRequest {
    pub category_id: Uuid,
    pub bbox: Vec<f64>,
    pub area: Option<f64>,
    pub iscrowd: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AnnotationResponse {
    pub annotation: AnnotationWithCategory,
}

#[derive(Debug, Deserialize)]
pub struct AnnotationsListResponse {
    pub annotations: Vec<AnnotationWithCategory>,
}

pub struct AnnotationsApi {
    client: ApiClient,
}

impl AnnotationsApi {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }

    pub async fn list_annotations(
        &self,
        jwt: &str,
        project_id: Uuid,
        task_id: Uuid,
    ) -> ApiResult<Vec<AnnotationWithCategory>> {
        let endpoint = format!("/projects/{}/tasks/{}/annotations", project_id, task_id);
        let response: AnnotationsListResponse = self.client.get(&endpoint, Some(jwt)).await?;
        Ok(response.annotations)
    }

    pub async fn create_annotation(
        &self,
        jwt: &str,
        project_id: Uuid,
        task_id: Uuid,
        request: &CreateAnnotationRequest,
    ) -> ApiResult<AnnotationWithCategory> {
        let endpoint = format!("/projects/{}/tasks/{}/annotations", project_id, task_id);
        let response: AnnotationResponse = self.client.post(&endpoint, request, Some(jwt)).await?;
        Ok(response.annotation)
    }

    pub async fn delete_annotation(
        &self,
        jwt: &str,
        project_id: Uuid,
        task_id: Uuid,
        annotation_id: Uuid,
    ) -> ApiResult<()> {
        let endpoint = format!("/projects/{}/tasks/{}/annotations/{}", project_id, task_id, annotation_id);
        self.client.delete(&endpoint, Some(jwt)).await
    }

    pub async fn save_annotations(
        &self,
        jwt: &str,
        project_id: Uuid,
        task_id: Uuid,
        annotations: &[CreateAnnotationRequest],
    ) -> ApiResult<Vec<AnnotationWithCategory>> {
        // First, clear existing annotations
        let existing = match self.list_annotations(jwt, project_id, task_id).await {
            Ok(annotations) => annotations,
            Err(e) => {
                // If listing fails, it might be because there are no annotations yet
                // Continue with empty list
                println!("Warning: Failed to list existing annotations: {}", e);
                Vec::new()
            }
        };
        
        for existing_annotation in existing {
            if let Err(e) = self.delete_annotation(jwt, project_id, task_id, existing_annotation.annotation_id).await {
                println!("Warning: Failed to delete annotation {}: {}", existing_annotation.annotation_id, e);
                // Continue with other deletions instead of failing completely
            }
        }
        
        // Create new annotations
        let mut saved_annotations = Vec::new();
        for (i, annotation_request) in annotations.iter().enumerate() {
            match self.create_annotation(jwt, project_id, task_id, annotation_request).await {
                Ok(saved) => saved_annotations.push(saved),
                Err(e) => {
                    println!("Failed to create annotation {}: {}", i, e);
                    return Err(e);
                }
            }
        }
        
        Ok(saved_annotations)
    }
}

impl Default for AnnotationsApi {
    fn default() -> Self {
        Self::new()
    }
}