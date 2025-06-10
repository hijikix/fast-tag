use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;

pub async fn download_image_bytes(url: &str) -> Option<Vec<u8>> {
    println!("Downloading from URL: {}", url);
    
    let response = match reqwest::get(url).await {
        Ok(response) => {
            println!("HTTP response status: {}", response.status());
            if !response.status().is_success() {
                eprintln!("HTTP error: {}", response.status());
                return None;
            }
            response
        },
        Err(e) => {
            eprintln!("Image download error: {}", e);
            return None;
        }
    };

    // Check content type
    if let Some(content_type) = response.headers().get("content-type") {
        if let Ok(content_type_str) = content_type.to_str() {
            println!("Content-Type: {}", content_type_str);
            if !content_type_str.starts_with("image/") {
                eprintln!("Warning: Content-Type is not an image type: {}", content_type_str);
            }
        }
    }

    match response.bytes().await {
        Ok(bytes) => {
            println!("Successfully downloaded {} bytes", bytes.len());
            Some(bytes.to_vec())
        },
        Err(e) => {
            eprintln!("Image bytes error: {}", e);
            None
        }
    }
}

pub fn load_image_from_url(url: &str) -> Result<image::DynamicImage, image::ImageError> {
    println!("Attempting to load image from URL: {}", url);
    
    // Check if URL is empty or invalid
    if url.is_empty() {
        eprintln!("Error: URL is empty");
        return Err(image::ImageError::Unsupported(
            image::error::UnsupportedError::from_format_and_kind(
                image::error::ImageFormatHint::Unknown,
                image::error::UnsupportedErrorKind::Format(image::error::ImageFormatHint::Unknown),
            ),
        ));
    }
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let image_bytes = rt.block_on(download_image_bytes(url));

    if let Some(bytes) = image_bytes {
        println!("Downloaded {} bytes from URL", bytes.len());
        
        // Try to guess the format from the first few bytes
        if bytes.len() < 16 {
            eprintln!("Error: Downloaded data is too small ({} bytes) to be a valid image", bytes.len());
            return Err(image::ImageError::Unsupported(
                image::error::UnsupportedError::from_format_and_kind(
                    image::error::ImageFormatHint::Unknown,
                    image::error::UnsupportedErrorKind::Format(image::error::ImageFormatHint::Unknown),
                ),
            ));
        }
        
        // Check for common image format headers
        let format_hint = if bytes.starts_with(b"\xFF\xD8\xFF") {
            "JPEG"
        } else if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
            "PNG"
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            "GIF"
        } else if bytes.starts_with(b"RIFF") && bytes.len() > 11 && &bytes[8..12] == b"WEBP" {
            "WebP"
        } else {
            "Unknown"
        };
        
        println!("Detected format: {}", format_hint);
        
        match image::load_from_memory(&bytes) {
            Ok(image) => {
                println!("Image loaded successfully! Dimensions: {}x{}", image.width(), image.height());
                Ok(image)
            }
            Err(e) => {
                eprintln!("Failed to decode image: {}", e);
                eprintln!("First 32 bytes: {:?}", &bytes[..bytes.len().min(32)]);
                Err(e)
            }
        }
    } else {
        eprintln!("Failed to download image bytes from URL");
        Err(image::ImageError::Unsupported(
            image::error::UnsupportedError::from_format_and_kind(
                image::error::ImageFormatHint::Unknown,
                image::error::UnsupportedErrorKind::Format(image::error::ImageFormatHint::Unknown),
            ),
        ))
    }
}

pub fn create_bevy_image_from_dynamic(dynamic_image: image::DynamicImage) -> Image {
    Image::from_dynamic(dynamic_image, true, RenderAssetUsages::default())
}

pub fn spawn_image_sprite(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    url: &str,
) -> Result<(Entity, Vec2), image::ImageError> {
    let dynamic_image = load_image_from_url(url)?;
    let width = dynamic_image.width() as f32;
    let height = dynamic_image.height() as f32;
    let dimensions = Vec2::new(width, height);
    
    let image = create_bevy_image_from_dynamic(dynamic_image);
    let image_handle = images.add(image);
    let image_entity = commands.spawn(Sprite::from_image(image_handle)).id();
    Ok((image_entity, dimensions))
}