use bevy::prelude::*;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy_egui::EguiContexts;
use crate::rectangle::{Rectangle, Corner};

#[derive(PartialEq, Default)]
pub enum InteractionMode {
    #[default]
    Default,
    Resizing,
    Drawing,
    Grabbing,
}

#[derive(Default)]
pub struct ResizingHandler {
    pub rectangle_index: Option<usize>,
    pub corner: Option<Corner>,
}

impl ResizingHandler {
    pub fn new() -> Self {
        Self {
            rectangle_index: None,
            corner: None,
        }
    }

    pub fn process(
        &mut self,
        rectangles: &mut Vec<Rectangle>,
        cursor_position: Option<Vec2>,
        mouse_events: &[MouseButtonInput],
        mode: &mut InteractionMode,
        selected_index: &mut Option<usize>,
        egui_contexts: &mut EguiContexts,
    ) {
        const MARGIN: f32 = 5.0;
        let ctx = egui_contexts.ctx_mut();

        if *mode == InteractionMode::Default {
            let mut hovering_index = None;
            let mut corner_option = None;

            for (index, rect) in rectangles.iter().enumerate() {
                if let Some(pos) = cursor_position {
                    if let Some(corner) = rect.get_corner_at_point(pos, MARGIN) {
                        hovering_index = Some(index);
                        corner_option = Some(corner);
                        break;
                    }
                }
            }

            if hovering_index.is_some() {
                ctx.set_cursor_icon(bevy_egui::egui::CursorIcon::Grab);

                for event in mouse_events.iter() {
                    if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
                        self.rectangle_index = hovering_index;
                        self.corner = corner_option;
                        *mode = InteractionMode::Resizing;
                    }
                }
            }
        }

        if *mode == InteractionMode::Resizing {
            if let (Some(pos), Some(rect_idx), Some(corner)) = (cursor_position, self.rectangle_index, self.corner) {
                if let Some(rectangle) = rectangles.get_mut(rect_idx) {
                    rectangle.resize_corner(corner, pos);
                }
            }
            ctx.set_cursor_icon(bevy_egui::egui::CursorIcon::Grabbing);

            for event in mouse_events.iter() {
                if event.button == MouseButton::Left && event.state == ButtonState::Released {
                    if let Some(rect_idx) = self.rectangle_index {
                        if let Some(rectangle) = rectangles.get_mut(rect_idx) {
                            rectangle.normalize_position();
                        }
                        *selected_index = Some(rect_idx);
                    }
                    self.clear();
                    *mode = InteractionMode::Default;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.rectangle_index = None;
        self.corner = None;
    }
}

#[derive(Default)]
pub struct GrabbingHandler {
    pub rectangle_index: Option<usize>,
    pub start_position: Option<Vec2>,
}

impl GrabbingHandler {
    pub fn new() -> Self {
        Self {
            rectangle_index: None,
            start_position: None,
        }
    }

    pub fn process(
        &mut self,
        rectangles: &mut Vec<Rectangle>,
        cursor_position: Option<Vec2>,
        mouse_events: &[MouseButtonInput],
        mode: &mut InteractionMode,
        selected_index: &mut Option<usize>,
        egui_contexts: &mut EguiContexts,
    ) {
        const MARGIN: f32 = 5.0;
        let ctx = egui_contexts.ctx_mut();

        if *mode == InteractionMode::Default {
            let mut hovering_index = None;

            for (index, rect) in rectangles.iter().enumerate() {
                if let Some(pos) = cursor_position {
                    if rect.contains_point(pos, MARGIN) {
                        hovering_index = Some(index);
                        break;
                    }
                }
            }

            if hovering_index.is_some() {
                ctx.set_cursor_icon(bevy_egui::egui::CursorIcon::Grab);
                for event in mouse_events.iter() {
                    if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
                        self.rectangle_index = hovering_index;
                        self.start_position = cursor_position;
                        *mode = InteractionMode::Grabbing;
                    }
                }
            }
        }

        if *mode == InteractionMode::Grabbing {
            if let (Some(start_pos), Some(current_pos), Some(rect_idx)) = 
                (self.start_position, cursor_position, self.rectangle_index) {
                let delta = current_pos - start_pos;
                if let Some(rectangle) = rectangles.get_mut(rect_idx) {
                    rectangle.move_by(delta);
                    self.start_position = Some(current_pos);
                }
            }
            ctx.set_cursor_icon(bevy_egui::egui::CursorIcon::Grabbing);

            for event in mouse_events.iter() {
                if event.button == MouseButton::Left && event.state == ButtonState::Released {
                    *selected_index = self.rectangle_index;
                    self.clear();
                    *mode = InteractionMode::Default;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.rectangle_index = None;
        self.start_position = None;
    }
}

#[derive(Default)]
pub struct DrawingHandler {
    pub start_position: Option<Vec2>,
}

impl DrawingHandler {
    pub fn new() -> Self {
        Self {
            start_position: None,
        }
    }

    pub fn process(
        &mut self,
        rectangles: &mut Vec<Rectangle>,
        cursor_position: Option<Vec2>,
        mouse_events: &[MouseButtonInput],
        mode: &mut InteractionMode,
        selected_class: usize,
        egui_input_use: bool,
        gizmos: &mut Gizmos,
    ) {
        if *mode == InteractionMode::Default && !egui_input_use && cursor_position.is_some() {
            for event in mouse_events.iter() {
                if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
                    self.start_position = cursor_position;
                    *mode = InteractionMode::Drawing;
                }
            }
        }

        if *mode == InteractionMode::Drawing && !egui_input_use && cursor_position.is_some() {
            for event in mouse_events.iter() {
                if self.start_position.is_some()
                    && event.button == MouseButton::Left
                    && event.state == ButtonState::Released
                {
                    let start_pos = self.start_position.unwrap();
                    let end_pos = cursor_position.unwrap();
                    
                    rectangles.push(Rectangle::new(selected_class, start_pos, end_pos));
                    self.clear();
                    *mode = InteractionMode::Default;
                }
            }
        }

        if *mode == InteractionMode::Drawing
            && self.start_position.is_some()
            && cursor_position.is_some()
        {
            let start_pos = self.start_position.unwrap();
            let end_pos = cursor_position.unwrap();
            gizmos.rect_2d(
                (start_pos + end_pos) / 2.0,
                end_pos - start_pos,
                crate::rectangle::rect_color(selected_class),
            );
        }
    }

    pub fn clear(&mut self) {
        self.start_position = None;
    }
}

pub fn key_code_to_class(keyboard: &Res<ButtonInput<KeyCode>>) -> Option<usize> {
    for (key, class) in [
        (KeyCode::Digit1, 1),
        (KeyCode::Digit2, 2),
        (KeyCode::Digit3, 3),
        (KeyCode::Digit4, 4),
        (KeyCode::Digit5, 5),
        (KeyCode::Digit6, 6),
        (KeyCode::Digit7, 7),
        (KeyCode::Digit8, 8),
        (KeyCode::Digit9, 9),
    ] {
        if keyboard.pressed(key) {
            return Some(class);
        }
    }
    None
}