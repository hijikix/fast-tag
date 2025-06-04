use bevy::prelude::*;
use crate::core::rectangle::Rectangle;

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Command {
    AddRectangle { rectangle: Rectangle },
    DeleteRectangle { index: usize, rectangle: Rectangle },
    MoveRectangle { index: usize, old_position: (f32, f32), new_position: (f32, f32) },
    ResizeRectangle { index: usize, old_rect: Rectangle, new_rect: Rectangle },
}

impl Command {
    pub fn execute(&self, rectangles: &mut Vec<Rectangle>) {
        match self {
            Command::AddRectangle { rectangle } => {
                rectangles.push(rectangle.clone());
            }
            Command::DeleteRectangle { index, .. } => {
                if *index < rectangles.len() {
                    rectangles.remove(*index);
                }
            }
            Command::MoveRectangle { index, new_position, .. } => {
                if let Some(rect) = rectangles.get_mut(*index) {
                    let size = rect.size();
                    let center = Vec2::new(new_position.0, new_position.1);
                    rect.position.0 = center - size / 2.0;
                    rect.position.1 = center + size / 2.0;
                }
            }
            Command::ResizeRectangle { index, new_rect, .. } => {
                if let Some(rect) = rectangles.get_mut(*index) {
                    *rect = new_rect.clone();
                }
            }
        }
    }

    pub fn undo(&self, rectangles: &mut Vec<Rectangle>) {
        match self {
            Command::AddRectangle { .. } => {
                rectangles.pop();
            }
            Command::DeleteRectangle { index, rectangle } => {
                rectangles.insert(*index, rectangle.clone());
            }
            Command::MoveRectangle { index, old_position, .. } => {
                if let Some(rect) = rectangles.get_mut(*index) {
                    let size = rect.size();
                    let center = Vec2::new(old_position.0, old_position.1);
                    rect.position.0 = center - size / 2.0;
                    rect.position.1 = center + size / 2.0;
                }
            }
            Command::ResizeRectangle { index, old_rect, .. } => {
                if let Some(rect) = rectangles.get_mut(*index) {
                    *rect = old_rect.clone();
                }
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct CommandHistory {
    commands: Vec<Command>,
    current_index: Option<usize>,
}

impl CommandHistory {
    pub fn push(&mut self, command: Command) {
        // Clear redo history when new command is pushed
        if let Some(index) = self.current_index {
            self.commands.truncate(index + 1);
        } else {
            self.commands.clear();
        }
        
        self.commands.push(command);
        self.current_index = Some(self.commands.len() - 1);
    }

    pub fn undo(&mut self, rectangles: &mut Vec<Rectangle>) -> bool {
        if let Some(index) = self.current_index {
            if let Some(command) = self.commands.get(index) {
                command.undo(rectangles);
                self.current_index = if index > 0 { Some(index - 1) } else { None };
                return true;
            }
        }
        false
    }

    pub fn redo(&mut self, rectangles: &mut Vec<Rectangle>) -> bool {
        let next_index = match self.current_index {
            Some(index) => index + 1,
            None => 0,
        };

        if let Some(command) = self.commands.get(next_index) {
            command.execute(rectangles);
            self.current_index = Some(next_index);
            return true;
        }
        false
    }
}