use super::{ApiError, ApiResult, ApiConfig};
use reqwest::{Client, Response};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    config: ApiConfig,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            config: ApiConfig::default(),
        }
    }


    async fn handle_response<T: DeserializeOwned>(response: Response) -> ApiResult<T> {
        let status = response.status();
        
        if status.is_success() {
            let body = response.text().await
                .map_err(|e| ApiError::NetworkError(format!("Failed to read response body: {}", e)))?;
            
            serde_json::from_str::<T>(&body).map_err(|e| {
                ApiError::ParseError(format!("Failed to parse response: {}. Response body: {}", e, body))
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            match status.as_u16() {
                401 => Err(ApiError::AuthenticationError(error_text)),
                400 => Err(ApiError::BadRequest(error_text)),
                404 => Err(ApiError::NotFound(error_text)),
                500..=599 => Err(ApiError::ServerError(error_text)),
                _ => Err(ApiError::Unknown(format!("HTTP {}: {}", status, error_text))),
            }
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, endpoint: &str, token: Option<&str>) -> ApiResult<T> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        let mut request = self.client.get(&url);
        
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        Self::handle_response(response).await
    }

    pub async fn post<T: DeserializeOwned, R: Serialize>(
        &self,
        endpoint: &str,
        body: &R,
        token: Option<&str>,
    ) -> ApiResult<T> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        let mut request = self.client.post(&url);
        
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.json(body).send().await?;
        Self::handle_response(response).await
    }

    pub async fn put<T: DeserializeOwned, R: Serialize>(
        &self,
        endpoint: &str,
        body: &R,
        token: Option<&str>,
    ) -> ApiResult<T> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        let mut request = self.client.put(&url);
        
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.json(body).send().await?;
        Self::handle_response(response).await
    }

    pub async fn delete(&self, endpoint: &str, token: Option<&str>) -> ApiResult<()> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        let mut request = self.client.delete(&url);
        
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            match status.as_u16() {
                401 => Err(ApiError::AuthenticationError(error_text)),
                400 => Err(ApiError::BadRequest(error_text)),
                404 => Err(ApiError::NotFound(error_text)),
                500..=599 => Err(ApiError::ServerError(error_text)),
                _ => Err(ApiError::Unknown(format!("HTTP {}: {}", status, error_text))),
            }
        }
    }

    pub async fn get_bytes(&self, url: &str) -> ApiResult<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        
        if response.status().is_success() {
            response.bytes().await
                .map(|bytes| bytes.to_vec())
                .map_err(|e| ApiError::NetworkError(format!("Failed to read bytes: {}", e)))
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServerError(format!("HTTP {}: {}", status, error_text)))
        }
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}