use super::{ApiClient, ApiResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

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

#[derive(Debug, Serialize, Clone)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub supercategory: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub coco_id: Option<i32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct UpdateCategoryRequest {
    pub name: String,
    pub supercategory: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub coco_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryResponse {
    pub category: AnnotationCategory,
}

#[derive(Debug, Deserialize)]
pub struct CategoriesListResponse {
    pub categories: Vec<AnnotationCategory>,
}

pub struct CategoriesApi {
    client: ApiClient,
}

impl CategoriesApi {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }

    pub async fn list_categories(&self, jwt: &str, project_id: Uuid) -> ApiResult<Vec<AnnotationCategory>> {
        let endpoint = format!("/projects/{}/image-annotation-categories", project_id);
        let response: CategoriesListResponse = self.client.get(&endpoint, Some(jwt)).await?;
        Ok(response.categories)
    }

    pub async fn create_category(
        &self,
        jwt: &str,
        project_id: Uuid,
        request: &CreateCategoryRequest,
    ) -> ApiResult<AnnotationCategory> {
        let endpoint = format!("/projects/{}/image-annotation-categories", project_id);
        let response: CategoryResponse = self.client.post(&endpoint, request, Some(jwt)).await?;
        Ok(response.category)
    }

    #[allow(dead_code)]
    pub async fn update_category(
        &self,
        jwt: &str,
        project_id: Uuid,
        category_id: Uuid,
        request: &UpdateCategoryRequest,
    ) -> ApiResult<AnnotationCategory> {
        let endpoint = format!("/projects/{}/image-annotation-categories/{}", project_id, category_id);
        let response: CategoryResponse = self.client.put(&endpoint, request, Some(jwt)).await?;
        Ok(response.category)
    }

    #[allow(dead_code)]
    pub async fn delete_category(
        &self,
        jwt: &str,
        project_id: Uuid,
        category_id: Uuid,
    ) -> ApiResult<()> {
        let endpoint = format!("/projects/{}/image-annotation-categories/{}", project_id, category_id);
        self.client.delete(&endpoint, Some(jwt)).await
    }
}

impl Default for CategoriesApi {
    fn default() -> Self {
        Self::new()
    }
}