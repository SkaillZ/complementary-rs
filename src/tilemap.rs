use std::{
    error::Error,
    fmt::{Debug, Display},
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use bytemuck::Contiguous;
use wgpu::include_wgsl;

use crate::{
    math::{Bounds, Color, Direction, FMat4, FVec2},
    rendering::{self, ColoredVertex, DrawState, UniformBuffer},
    window::DrawContext,
};

#[derive(Clone, Copy, Debug, Contiguous)]
#[repr(u8)]
pub enum Tile {
    Air,
    Solid,

    SpikesLeft,
    SpikesRight,
    SpikesUp,
    SpikesDown,

    SpawnPoint,

    GoalLeft,
    GoalRight,
    GoalUp,
    GoalDown,

    SpikeAllSides,
}

impl Tile {
    fn spawn(&self) {}

    pub fn is_solid(&self) -> bool {
        match self {
            Tile::Air => false,
            Tile::Solid => true,
            Tile::SpikesLeft => true,
            Tile::SpikesRight => true,
            Tile::SpikesUp => true,
            Tile::SpikesDown => true,
            Tile::SpawnPoint => false,
            Tile::GoalLeft => false,
            Tile::GoalRight => false,
            Tile::GoalUp => false,
            Tile::GoalDown => false,
            Tile::SpikeAllSides => false,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Tile::Air => Color::WHITE,
            Tile::Solid => Color::BLACK,
            Tile::SpikesLeft => Color::RED,
            Tile::SpikesRight => Color::RED,
            Tile::SpikesUp => Color::RED,
            Tile::SpikesDown => Color::RED,
            Tile::SpawnPoint => Color::GREEN,
            Tile::GoalLeft => Color::ORANGE,
            Tile::GoalRight => Color::ORANGE,
            Tile::GoalUp => Color::ORANGE,
            Tile::GoalDown => Color::ORANGE,
            Tile::SpikeAllSides => Color::RED,
        }
    }

    pub fn direction(&self) -> Option<Direction> {
        match self {
            Tile::Air => None,
            Tile::Solid => None,
            Tile::SpikesLeft => Some(Direction::Left),
            Tile::SpikesRight => Some(Direction::Right),
            Tile::SpikesUp => Some(Direction::Up),
            Tile::SpikesDown => Some(Direction::Down),
            Tile::SpawnPoint => None,
            Tile::GoalLeft => Some(Direction::Left),
            Tile::GoalRight => Some(Direction::Right),
            Tile::GoalUp => Some(Direction::Up),
            Tile::GoalDown => Some(Direction::Down),
            Tile::SpikeAllSides => None,
        }
    }
}

pub struct Tilemap {
    width: i32,
    height: i32,
    tiles: Vec<Tile>,
}

impl Tilemap {
    pub fn new(width: i32, height: i32) -> Tilemap {
        assert!(width > 0 && height > 0);
        Self {
            width,
            height,
            tiles: vec![Tile::Air; (width * height) as usize],
        }
    }

    pub fn load_from_file<T: AsRef<Path>>(path: T) -> Result<Tilemap, TilemapLoadError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        if &buf != b"CMTM" {
            return Err(TilemapLoadError::InvalidMagic);
        }

        reader.read_exact(&mut buf)?;
        let width = i32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let height = i32::from_le_bytes(buf);

        let mut bytes = vec![0; (width * height) as usize];
        reader.read_exact(&mut bytes[..])?;

        let tiles: Vec<Tile> = bytes
            .into_iter()
            .map(|byte| Tile::from_integer(byte).unwrap_or(Tile::Air))
            .collect();

        Ok(Tilemap {
            width,
            height,
            tiles,
        })
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Tile {
        self.tiles[(self.width * y + x) as usize]
    }

    pub fn set_tile(&mut self, x: i32, y: i32, tile: Tile) {
        self.tiles[(self.width * y + x) as usize] = tile;
        tile.spawn();
    }

    pub fn get_spawn_point(&self) -> Option<FVec2> {
        for y in 0..self.height {
            for x in 0..self.width {
                if matches!(self.get_tile(x, y), Tile::SpawnPoint) {
                    return Some(FVec2::new(x as f32, y as f32));
                }
            }
        }

        None
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn contains_bounds(&self, bounds: Bounds) -> bool {
        bounds.min.x >= 0.0
            || bounds.min.y >= 0.0
            || bounds.max.x < self.width as f32
            || bounds.max.y < self.height as f32
    }
}

impl Default for Tilemap {
    fn default() -> Self {
        Self::new(48, 27)
    }
}

pub struct TilemapRenderer {
    vertex_count: usize,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: UniformBuffer<TilemapUniforms>,
    render_pipeline: wgpu::RenderPipeline,
}

impl TilemapRenderer {
    pub fn new(device: &wgpu::Device, tilemap: &Tilemap) -> TilemapRenderer {
        let uniform_buffer = UniformBuffer::new(device, "tilemap_uniforms");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniform_buffer.bind_group_layout()],
            label: Some("tilemap_pipeline_layout"),
            push_constant_ranges: &[],
        });

        let (vertices, max_byte_size_bytes) = TilemapRenderer::get_tilemap_vertices(tilemap);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tilemap_vertex_buffer"),
            size: max_byte_size_bytes as _,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let current_length = vertices.len() * std::mem::size_of::<ColoredVertex>();
        vertex_buffer.slice(..).get_mapped_range_mut()[..current_length as usize]
            .copy_from_slice(bytemuck::cast_slice(&vertices));
        vertex_buffer.unmap();

        let render_pipeline =
            device.create_render_pipeline(&rendering::create_pipeline_descriptor(
                Some("player_pipeline"),
                &device.create_shader_module(&include_wgsl!("shaders/tilemap.wgsl")),
                Some(&pipeline_layout),
                &[ColoredVertex::layout()],
            ));

        TilemapRenderer {
            vertex_count: vertices.len(),
            vertex_buffer,
            uniform_buffer,
            render_pipeline,
        }
    }

    fn get_tilemap_vertices(tilemap: &Tilemap) -> (Vec<ColoredVertex>, usize) {
        // Each tile has six vertices max.
        let max_size = tilemap.width() as usize
            * tilemap.height() as usize
            * std::mem::size_of::<ColoredVertex>()
            * 6;
        let mut vertices = Vec::with_capacity((max_size / 3) as usize);

        for y in 0..tilemap.height() {
            for x in 0..tilemap.width() {
                let tile = tilemap.get_tile(x, y);
                let x = x as f32;
                let y = y as f32;
                let color = tile.color();
                vertices.push(ColoredVertex::new(FVec2::new(x + 0.0, y + 1.0), color));
                vertices.push(ColoredVertex::new(FVec2::new(x + 0.0, y + 0.0), color));
                vertices.push(ColoredVertex::new(FVec2::new(x + 1.0, y + 1.0), color));
                vertices.push(ColoredVertex::new(FVec2::new(x + 1.0, y + 1.0), color));
                vertices.push(ColoredVertex::new(FVec2::new(x + 0.0, y + 0.0), color));
                vertices.push(ColoredVertex::new(FVec2::new(x + 1.0, y + 0.0), color));
            }
        }

        (vertices, max_size)
    }

    pub fn draw(&mut self, context: &mut DrawContext, state: &DrawState) {
        let uniforms = TilemapUniforms {
            view_matrix: state.view_matrix,
        };
        self.uniform_buffer
            .write_with_queue(context.queue, uniforms);

        let mut rpass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &context.output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
                label: None,
            });
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_bind_group(0, &self.uniform_buffer.bind_group(), &[]);
        rpass.draw(0..self.vertex_count as u32, 0..1);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TilemapUniforms {
    view_matrix: FMat4,
}

#[derive(Debug)]
pub enum TilemapLoadError {
    Io(io::Error),
    InvalidMagic,
}

impl From<io::Error> for TilemapLoadError {
    fn from(inner: io::Error) -> Self {
        TilemapLoadError::Io(inner)
    }
}

impl Display for TilemapLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TilemapLoadError::Io(err) => write!(f, "IO error: {err}"),
            TilemapLoadError::InvalidMagic => write!(f, "Invalid file magic"),
        }
    }
}
impl Error for TilemapLoadError {}
