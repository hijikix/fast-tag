use crate::pages::components::egui_common;
use crate::state::AppState;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

#[derive(Resource, Default)]
pub struct Parameters {
    pub url: String,
}

fn load_image(url: &str) -> Result<image::DynamicImage, image::ImageError> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let image_bytes = rt.block_on(async {
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
    });

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

pub fn setup(mut commands: Commands, params: Res<Parameters>, mut images: ResMut<Assets<Image>>) {
    println!("detail setup");
    println!("url {:?}", params.url);
    let dynamic_image = match load_image(&params.url) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("load_image error: {}", e);
            return;
        }
    };
    let image = Image::from_dynamic(dynamic_image, true, RenderAssetUsages::default());
    let image_handle = images.add(image);
    commands.spawn(Sprite::from_image(image_handle));
}

pub fn update() {}

pub fn ui_system(
    mut contexts: EguiContexts,
    current_state: Res<State<AppState>>,
    next_state: ResMut<NextState<AppState>>,
) {
    egui_common::ui_top_panel(&mut contexts, current_state, next_state);
}

pub fn cleanup() {
    println!("detail cleanup");
}
