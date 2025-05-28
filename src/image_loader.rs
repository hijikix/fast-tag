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

    match response.bytes().await {
        Ok(bytes) => Some(bytes.to_vec()),
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
        let image = image::load_from_memory(&bytes)?;
        println!("image loaded!!!");
        return Ok(image);
    }

    Err(image::ImageError::Unsupported(
        image::error::UnsupportedError::from_format_and_kind(
            image::error::ImageFormatHint::Unknown,
            image::error::UnsupportedErrorKind::Format(image::error::ImageFormatHint::Unknown),
        ),
    ))
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