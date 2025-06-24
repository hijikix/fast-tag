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
pub struct BoundingBox {
    pub category_id: Uuid,
    pub bbox: Vec<f64>,
    pub area: Option<f64>,
    pub iscrowd: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CreateAnnotationRequest {
    pub bboxes: Vec<BoundingBox>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AnnotationResponse {
    pub annotations: Vec<AnnotationWithCategory>,
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

    #[allow(dead_code)]
    pub async fn list_annotations(
        &self,
        jwt: &str,
        project_id: Uuid,
        task_id: Uuid,
    ) -> ApiResult<Vec<AnnotationWithCategory>> {
        self.list_annotations_with_options(jwt, project_id, task_id, false).await
    }

    pub async fn list_annotations_with_options(
        &self,
        jwt: &str,
        project_id: Uuid,
        task_id: Uuid,
        latest_only: bool,
    ) -> ApiResult<Vec<AnnotationWithCategory>> {
        let mut endpoint = format!("/projects/{}/tasks/{}/annotations", project_id, task_id);
        if latest_only {
            endpoint.push_str("?latest_only=true");
        }
        let response: AnnotationsListResponse = self.client.get(&endpoint, Some(jwt)).await?;
        Ok(response.annotations)
    }

    pub async fn create_annotation(
        &self,
        jwt: &str,
        project_id: Uuid,
        task_id: Uuid,
        request: &CreateAnnotationRequest,
    ) -> ApiResult<Vec<AnnotationWithCategory>> {
        let endpoint = format!("/projects/{}/tasks/{}/annotations", project_id, task_id);
        let response: AnnotationResponse = self.client.post(&endpoint, request, Some(jwt)).await?;
        Ok(response.annotations)
    }

    #[allow(dead_code)]
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
        bounding_boxes: &[BoundingBox],
    ) -> ApiResult<Vec<AnnotationWithCategory>> {
        // Create new annotations without deleting existing ones to preserve history
        if bounding_boxes.is_empty() {
            return Ok(Vec::new());
        }
        
        let request = CreateAnnotationRequest {
            bboxes: bounding_boxes.to_vec(),
            metadata: None,
        };
        
        match self.create_annotation(jwt, project_id, task_id, &request).await {
            Ok(saved) => Ok(saved),
            Err(e) => {
                println!("Failed to create annotations: {}", e);
                Err(e)
            }
        }
    }
}

impl Default for AnnotationsApi {
    fn default() -> Self {
        Self::new()
    }
}