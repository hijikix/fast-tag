use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Parameters {
    pub url: String,
}

pub fn setup(params: Res<Parameters>) {
    println!("detail setup");
    println!("url {:?}", params.url);
}

pub fn update() {
    println!("detail update");
}

pub fn cleanup() {
    println!("detail cleanup");
}
