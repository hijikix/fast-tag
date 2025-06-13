use super::{ApiClient, ApiResult, ApiError};

pub struct ResourcesApi {
    client: ApiClient,
}

impl ResourcesApi {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }

    pub async fn download_image(&self, url: &str) -> ApiResult<Vec<u8>> {
        if url.is_empty() {
            return Err(ApiError::BadRequest("URL is empty".to_string()));
        }

        let bytes = self.client.get_bytes(url).await?;
        
        // Basic validation of image data
        if bytes.len() < 16 {
            return Err(ApiError::ParseError(format!(
                "Downloaded data is too small ({} bytes) to be a valid image",
                bytes.len()
            )));
        }

        // Check for common image format headers
        let is_valid_image = bytes.starts_with(b"\xFF\xD8\xFF")         // JPEG
            || bytes.starts_with(b"\x89PNG\r\n\x1A\n")                 // PNG
            || bytes.starts_with(b"GIF87a")                             // GIF87a
            || bytes.starts_with(b"GIF89a")                             // GIF89a
            || (bytes.starts_with(b"RIFF") && bytes.len() > 11 && &bytes[8..12] == b"WEBP"); // WebP

        if !is_valid_image {
            return Err(ApiError::ParseError(
                "Downloaded data does not appear to be a valid image format".to_string()
            ));
        }

        Ok(bytes)
    }

    #[allow(dead_code)]
    pub async fn get_image_info(&self, url: &str) -> ApiResult<ImageInfo> {
        let bytes = self.download_image(url).await?;
        
        let format = if bytes.starts_with(b"\xFF\xD8\xFF") {
            ImageFormat::Jpeg
        } else if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
            ImageFormat::Png
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            ImageFormat::Gif
        } else if bytes.starts_with(b"RIFF") && bytes.len() > 11 && &bytes[8..12] == b"WEBP" {
            ImageFormat::WebP
        } else {
            ImageFormat::Unknown
        };

        Ok(ImageInfo {
            size: bytes.len(),
            format,
        })
    }

    #[allow(dead_code)]
    pub async fn validate_image_url(&self, url: &str) -> ApiResult<bool> {
        match self.download_image(url).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl Default for ResourcesApi {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ImageInfo {
    pub size: usize,
    pub format: ImageFormat,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Unknown,
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFormat::Jpeg => write!(f, "JPEG"),
            ImageFormat::Png => write!(f, "PNG"),
            ImageFormat::Gif => write!(f, "GIF"),
            ImageFormat::WebP => write!(f, "WebP"),
            ImageFormat::Unknown => write!(f, "Unknown"),
        }
    }
}