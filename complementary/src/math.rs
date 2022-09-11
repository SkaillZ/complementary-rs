use std::ops::{Mul, MulAssign};

pub use cgmath::*;
use serde::Deserialize;

pub type FVec2 = Vector2<f32>;
pub type FVec3 = Vector3<f32>;
pub type IVec2 = Vector2<i32>;
pub type IVec3 = Vector3<i32>;
pub type FMat4 = Matrix4<f32>;

#[derive(Debug, Copy, Clone)]
pub struct Bounds {
    pub min: FVec2,
    pub max: FVec2,
}

impl Bounds {
    pub fn new(min: FVec2, max: FVec2) -> Self {
        Self { min, max }
    }

    pub fn overlaps(&self, other: &Bounds) -> bool {
        return self.min.x < other.max.x && self.max.x > other.min.x &&
           self.min.y < other.max.y && self.max.y > other.min.y
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Deserialize)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color::new_solid(1.0, 1.0, 1.0);
    pub const GRAY: Color = Color::new_solid(0.5, 0.5, 0.5);
    pub const DARK_GRAY: Color = Color::new_solid(0.33, 0.33, 0.33);
    pub const LIGHT_GRAY: Color = Color::new_solid(0.76, 0.76, 0.76);
    pub const BLACK: Color = Color::new_solid(0.0, 0.0, 0.0);
    pub const RED: Color = Color::new_solid(1.0, 0.0, 0.0);
    pub const PINK: Color = Color::new_solid(1.0, 0.69, 0.69);
    pub const ORANGE: Color = Color::new_solid(1.0, 0.79, 0.0);
    pub const YELLOW: Color = Color::new_solid(1.0, 1.0, 0.0);
    pub const GREEN: Color = Color::new_solid(0.0, 1.0, 1.0);
    pub const MAGENTA: Color = Color::new_solid(1.0, 0.0, 1.0);
    pub const CYAN: Color = Color::new_solid(0.0, 1.0, 1.0);
    pub const BLUE: Color = Color::new_solid(0.0, 0.0, 1.0);
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn new_solid(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { r: self.r, g: self.g, b: self.b, a }
    }
}

impl From<u32> for Color {
    fn from(val: u32) -> Self {
        let r = val & 0xFF;
        let g = (val >> 8) & 0xFF;
        let b = (val >> 16) & 0xFF;
        let a = (val >> 24) & 0xFF;

        Self {
            r: (r as f32 / 255.0),
            g: (g as f32 / 255.0),
            b: (b as f32 / 255.0),
            a: (a as f32 / 255.0),
        }
    }
}

impl Mul for Color {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self { r: self.r * rhs.r, g: self.g * rhs.g, b: self.b * rhs.b, a: self.a * rhs.a }
    }
}

impl MulAssign for Color {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub const ALL: [Self; 4] = [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ];

    pub fn as_vec(self) -> FVec2 {
        match self {
            Direction::Left => FVec2::new(-1.0, 0.0),
            Direction::Right => FVec2::new(1.0, 0.0),
            Direction::Up => FVec2::new(0.0, -1.0),
            Direction::Down => FVec2::new(0.0, 1.0),
        }
    }

    pub fn inverse(self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}
