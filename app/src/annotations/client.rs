use super::*;

#[derive(Deserialize)]
struct AnnotationResponse {
    annotation: AnnotationWithCategory,
}

#[derive(Deserialize)]
struct AnnotationsListResponse {
    annotations: Vec<AnnotationWithCategory>,
}

#[derive(Deserialize)]
struct CategoryResponse {
    category: AnnotationCategory,
}

#[derive(Deserialize)]
struct CategoriesListResponse {
    categories: Vec<AnnotationCategory>,
}

pub async fn save_annotations(
    project_id: Uuid,
    task_id: Uuid,
    annotations: Vec<CreateAnnotationRequest>,
    token: String,
    api_base_url: String,
) -> Result<Vec<AnnotationWithCategory>, String> {
    let client = reqwest::Client::new();
    let mut saved_annotations = Vec::new();
    
    // First, clear existing annotations for this task
    let list_url = format!("{}/projects/{}/tasks/{}/annotations", api_base_url, project_id, task_id);
    let existing_response = client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to get existing annotations: {}", e))?;
    
    if existing_response.status().is_success() {
        let response_text = existing_response.text().await
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        
        if response_text.trim().is_empty() {
            // Handle empty response - no existing annotations
        } else {
            let existing_list: AnnotationsListResponse = serde_json::from_str(&response_text)
                .map_err(|e| format!("Failed to parse existing annotations: {}. Response body: {}", e, response_text))?;
            
            // Delete existing annotations
            for existing_annotation in existing_list.annotations {
                let delete_url = format!(
                    "{}/projects/{}/tasks/{}/annotations/{}", 
                    api_base_url, project_id, task_id, existing_annotation.id
                );
                let _delete_response = client
                    .delete(&delete_url)
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await
                    .map_err(|e| format!("Failed to delete existing annotation: {}", e))?;
            }
        }
    }
    
    // Create new annotations
    for annotation_request in annotations {
        let create_url = format!("{}/projects/{}/tasks/{}/annotations", api_base_url, project_id, task_id);
        let response = client
            .post(&create_url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&annotation_request)
            .send()
            .await
            .map_err(|e| format!("Failed to create annotation: {}", e))?;
        
        if response.status().is_success() {
            let response_text = response.text().await
                .map_err(|e| format!("Failed to read response body: {}", e))?;
            
            let annotation_response: AnnotationResponse = serde_json::from_str(&response_text)
                .map_err(|e| format!("Failed to parse annotation response: {}. Response body: {}", e, response_text))?;
            saved_annotations.push(annotation_response.annotation);
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Failed to create annotation with status {}: {}", status, error_text));
        }
    }
    
    Ok(saved_annotations)
}

pub async fn load_annotations(
    project_id: Uuid,
    task_id: Uuid,
    token: String,
    api_base_url: String,
) -> Result<Vec<AnnotationWithCategory>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/projects/{}/tasks/{}/annotations", api_base_url, project_id, task_id);
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to load annotations: {}", e))?;
    
    if response.status().is_success() {
        let annotations_list: AnnotationsListResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse annotations: {}", e))?;
        Ok(annotations_list.annotations)
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to load annotations with status {}: {}", status, error_text))
    }
}

pub async fn load_categories(
    project_id: Uuid,
    token: String,
    api_base_url: String,
) -> Result<Vec<AnnotationCategory>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/projects/{}/image-annotation-categories", api_base_url, project_id);
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to load categories: {}", e))?;
    
    if response.status().is_success() {
        let categories_list: CategoriesListResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse categories: {}", e))?;
        Ok(categories_list.categories)
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to load categories with status {}: {}", status, error_text))
    }
}

pub async fn create_category(
    project_id: Uuid,
    request: CreateCategoryRequest,
    token: String,
    api_base_url: String,
) -> Result<AnnotationCategory, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/projects/{}/image-annotation-categories", api_base_url, project_id);
    
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to create category: {}", e))?;
    
    if response.status().is_success() {
        let category_response: CategoryResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse category response: {}", e))?;
        Ok(category_response.category)
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to create category with status {}: {}", status, error_text))
    }
}