use bevy::prelude::*;
use bevy::color::palettes::css::*;

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub class: usize,
    pub position: (Vec2, Vec2),
}

impl Rectangle {
    pub fn new(class: usize, start: Vec2, end: Vec2) -> Self {
        let mut rect = Self {
            class,
            position: (start, end),
        };
        rect.normalize_position();
        rect
    }

    pub fn normalize_position(&mut self) {
        let (pos1, pos2) = &mut self.position;
        let min_x = pos1.x.min(pos2.x);
        let max_x = pos1.x.max(pos2.x);
        let min_y = pos1.y.min(pos2.y);
        let max_y = pos1.y.max(pos2.y);

        *pos1 = Vec2::new(min_x, min_y);
        *pos2 = Vec2::new(max_x, max_y);
    }

    pub fn center(&self) -> Vec2 {
        let (start, end) = self.position;
        (start + end) / 2.0
    }

    pub fn size(&self) -> Vec2 {
        let (start, end) = self.position;
        end - start
    }

    pub fn contains_point(&self, point: Vec2, margin: f32) -> bool {
        let (pos1, pos2) = self.position;
        let min_x = pos1.x.min(pos2.x);
        let max_x = pos1.x.max(pos2.x);
        let min_y = pos1.y.min(pos2.y);
        let max_y = pos1.y.max(pos2.y);

        let near_horizontal_edge = (point.y >= min_y - margin && point.y <= min_y + margin)
            || (point.y >= max_y - margin && point.y <= max_y + margin);

        let near_vertical_edge = (point.x >= min_x - margin && point.x <= min_x + margin)
            || (point.x >= max_x - margin && point.x <= max_x + margin);

        let within_x_range = point.x >= min_x && point.x <= max_x;
        let within_y_range = point.y >= min_y && point.y <= max_y;

        (near_horizontal_edge && within_x_range) || (near_vertical_edge && within_y_range)
    }

    pub fn get_corner_at_point(&self, point: Vec2, margin: f32) -> Option<Corner> {
        let (pos1, pos2) = self.position;
        let min_x = pos1.x.min(pos2.x);
        let max_x = pos1.x.max(pos2.x);
        let min_y = pos1.y.min(pos2.y);
        let max_y = pos1.y.max(pos2.y);

        let bottom_left = Vec2::new(min_x, min_y);
        let top_right = Vec2::new(max_x, max_y);
        let bottom_right = Vec2::new(max_x, min_y);
        let top_left = Vec2::new(min_x, max_y);

        if (point - bottom_left).length() <= margin {
            Some(Corner::BottomLeft)
        } else if (point - top_right).length() <= margin {
            Some(Corner::TopRight)
        } else if (point - bottom_right).length() <= margin {
            Some(Corner::BottomRight)
        } else if (point - top_left).length() <= margin {
            Some(Corner::TopLeft)
        } else {
            None
        }
    }

    pub fn move_by(&mut self, delta: Vec2) {
        self.position.0 += delta;
        self.position.1 += delta;
    }

    pub fn resize_corner(&mut self, corner: Corner, new_position: Vec2) {
        let (start_pos, end_pos) = &mut self.position;
        match corner {
            Corner::BottomLeft => *start_pos = new_position,
            Corner::BottomRight => {
                *start_pos = Vec2::new(start_pos.x, new_position.y);
                *end_pos = Vec2::new(new_position.x, end_pos.y);
            }
            Corner::TopLeft => {
                *start_pos = Vec2::new(new_position.x, start_pos.y);
                *end_pos = Vec2::new(end_pos.x, new_position.y);
            }
            Corner::TopRight => *end_pos = new_position,
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Corner {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

pub fn rect_color(class: usize) -> impl Into<Color> {
    match class {
        1 => RED,
        2 => BLUE,
        3 => GREEN,
        4 => YELLOW,
        5 => PURPLE,
        6 => AQUA,
        7 => BROWN,
        8 => NAVY,
        9 => LIME,
        _ => BLACK,
    }
}