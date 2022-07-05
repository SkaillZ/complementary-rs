pub use cgmath::*;

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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color::from_int(0xFFFFFFFF);
    pub const GRAY: Color = Color::from_int(0xFF808080);
    pub const DARK_GRAY: Color = Color::from_int(0xFF555555);
    pub const LIGHT_GRAY: Color = Color::from_int(0xFFC3C3C3);
    pub const BLACK: Color = Color::from_int(0xFF000000);
    pub const RED: Color = Color::from_int(0xFF0000FF);
    pub const PINK: Color = Color::from_int(0xFFAFAFFF);
    pub const ORANGE: Color = Color::from_int(0xFF00C8FF);
    pub const YELLOW: Color = Color::from_int(0xFF00FFFF);
    pub const GREEN: Color = Color::from_int(0xFF00FF00);
    pub const MAGENTA: Color = Color::from_int(0xFFFF00FF);
    pub const CYAN: Color = Color::from_int(0xFFFFFF00);
    pub const BLUE: Color = Color::from_int(0xFFFF0000);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn new_solid(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    // Const version of `from` trait impl (const impls are unstable, see https://github.com/rust-lang/rust/issues/67792)
    pub const fn from_int(val: u32) -> Self {
        let r = val & 0xFF;
        let g = (val >> 8) & 0xFF;
        let b = (val >> 16) & 0xFF;
        let a = (val >> 24) & 0xFF;

        Self {
            r: r as f32,
            g: g as f32,
            b: b as f32,
            a: a as f32,
        }
    }
}

impl From<u32> for Color {
    fn from(val: u32) -> Self {
        Self::from_int(val)
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
