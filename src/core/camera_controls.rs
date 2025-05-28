use bevy::prelude::*;
use bevy::input::ButtonState;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::window::PrimaryWindow;

pub struct CameraController {
    pub zoom_level: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub is_panning: bool,
    pub panning_start_screen_position: Option<Vec2>,
    pub camera_start_position: Option<Vec3>,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            zoom_level: 1.0,
            min_zoom: 0.1,
            max_zoom: 10.0,
            is_panning: false,
            panning_start_screen_position: None,
            camera_start_position: None,
        }
    }
}

impl CameraController {
    pub fn process_zoom(
        &mut self,
        mouse_wheel_events: &mut EventReader<MouseWheel>,
        cameras: &mut Query<&mut Transform, With<Camera>>,
        egui_input_use: bool,
    ) {
        if egui_input_use {
            return;
        }

        for event in mouse_wheel_events.read() {
            let zoom_delta = event.y * 0.001;
            let new_zoom = (self.zoom_level + zoom_delta).clamp(self.min_zoom, self.max_zoom);

            if new_zoom != self.zoom_level {
                self.zoom_level = new_zoom;

                if let Ok(mut camera_transform) = cameras.single_mut() {
                    camera_transform.scale = Vec3::splat(1.0 / self.zoom_level);
                }
            }
        }
    }

    pub fn process_panning(
        &mut self,
        mouse_button_events: &[MouseButtonInput],
        cameras: &mut Query<&mut Transform, With<Camera>>,
        q_window: Query<&Window, With<PrimaryWindow>>,
        egui_input_use: bool,
    ) {
        if egui_input_use {
            return;
        }

        let window = q_window.single().unwrap();
        let current_screen_pos = window.cursor_position();

        for event in mouse_button_events.iter() {
            if event.button == MouseButton::Right {
                match event.state {
                    ButtonState::Pressed => {
                        if !self.is_panning {
                            self.is_panning = true;
                            self.panning_start_screen_position = current_screen_pos;

                            if let Ok(camera_transform) = cameras.single() {
                                self.camera_start_position = Some(camera_transform.translation);
                            }
                        }
                    }
                    ButtonState::Released => {
                        self.reset_panning();
                    }
                }
            }
        }

        if self.is_panning {
            if let (Some(start_screen_pos), Some(current_screen_pos), Some(camera_start)) = (
                self.panning_start_screen_position,
                current_screen_pos,
                self.camera_start_position,
            ) {
                let screen_delta = current_screen_pos - start_screen_pos;
                let world_delta = screen_delta / self.zoom_level;

                if let Ok(mut camera_transform) = cameras.single_mut() {
                    camera_transform.translation =
                        camera_start - Vec3::new(world_delta.x, -world_delta.y, 0.0);
                }
            }
        }
    }

    pub fn reset_panning(&mut self) {
        self.is_panning = false;
        self.panning_start_screen_position = None;
        self.camera_start_position = None;
    }

}