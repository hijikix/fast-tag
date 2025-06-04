use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;

pub async fn download_image_bytes(url: &str) -> Option<Vec<u8>> {
    let response = match reqwest::get(url).await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("image download error: {}", e);
            return None;
        }
    };

    if !response.status().is_success() {
        eprintln!("HTTP error when downloading image: {}", response.status());
        return None;
    }

    match response.bytes().await {
        Ok(bytes) => {
            println!("Downloaded {} bytes from {}", bytes.len(), url);
            Some(bytes.to_vec())
        },
        Err(e) => {
            eprintln!("image bytes error: {}", e);
            None
        }
    }
}

pub async fn download_image_bytes_with_auth(url: &str, jwt: &str) -> Option<Vec<u8>> {
    let client = reqwest::Client::new();
    let response = match client
        .get(url)
        .bearer_auth(jwt)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            eprintln!("image download error: {}", e);
            return None;
        }
    };

    if !response.status().is_success() {
        eprintln!("HTTP error when downloading image: {}", response.status());
        return None;
    }

    match response.bytes().await {
        Ok(bytes) => {
            println!("Downloaded {} bytes from {}", bytes.len(), url);
            Some(bytes.to_vec())
        },
        Err(e) => {
            eprintln!("image bytes error: {}", e);
            None
        }
    }
}

pub fn load_image_from_url(url: &str) -> Result<image::DynamicImage, image::ImageError> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let image_bytes = rt.block_on(download_image_bytes(url));

    if let Some(bytes) = image_bytes {
        // Try to detect the image format from the first few bytes
        if bytes.len() < 10 {
            eprintln!("Image data too short: {} bytes", bytes.len());
            return Err(image::ImageError::Unsupported(
                image::error::UnsupportedError::from_format_and_kind(
                    image::error::ImageFormatHint::Unknown,
                    image::error::UnsupportedErrorKind::Format(image::error::ImageFormatHint::Unknown),
                ),
            ));
        }
        
        println!("First 10 bytes: {:?}", &bytes[0..10]);
        
        match image::load_from_memory(&bytes) {
            Ok(image) => {
                println!("image loaded!!!");
                Ok(image)
            },
            Err(e) => {
                eprintln!("Failed to parse image data: {}", e);
                Err(e)
            }
        }
    } else {
        eprintln!("No image bytes downloaded");
        Err(image::ImageError::Unsupported(
            image::error::UnsupportedError::from_format_and_kind(
                image::error::ImageFormatHint::Unknown,
                image::error::UnsupportedErrorKind::Format(image::error::ImageFormatHint::Unknown),
            ),
        ))
    }
}

pub fn load_image_from_url_with_auth(url: &str, jwt: &str) -> Result<image::DynamicImage, image::ImageError> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let image_bytes = rt.block_on(download_image_bytes_with_auth(url, jwt));

    if let Some(bytes) = image_bytes {
        // Try to detect the image format from the first few bytes
        if bytes.len() < 10 {
            eprintln!("Image data too short: {} bytes", bytes.len());
            return Err(image::ImageError::Unsupported(
                image::error::UnsupportedError::from_format_and_kind(
                    image::error::ImageFormatHint::Unknown,
                    image::error::UnsupportedErrorKind::Format(image::error::ImageFormatHint::Unknown),
                ),
            ));
        }
        
        println!("First 10 bytes: {:?}", &bytes[0..10]);
        
        match image::load_from_memory(&bytes) {
            Ok(image) => {
                println!("image loaded with auth!!!");
                Ok(image)
            },
            Err(e) => {
                eprintln!("Failed to parse image data: {}", e);
                Err(e)
            }
        }
    } else {
        eprintln!("No image bytes downloaded");
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
) -> Result<Entity, image::ImageError> {
    let dynamic_image = load_image_from_url(url)?;
    let image = create_bevy_image_from_dynamic(dynamic_image);
    let image_handle = images.add(image);
    let image_entity = commands.spawn(Sprite::from_image(image_handle)).id();
    Ok(image_entity)
}

pub fn spawn_image_sprite_with_auth(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    url: &str,
    jwt: &str,
) -> Result<Entity, image::ImageError> {
    let dynamic_image = load_image_from_url_with_auth(url, jwt)?;
    let image = create_bevy_image_from_dynamic(dynamic_image);
    let image_handle = images.add(image);
    let image_entity = commands.spawn(Sprite::from_image(image_handle)).id();
    Ok(image_entity)
}