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
    game::WorldType,
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
            Tile::SpikeAllSides => true,
        }
    }

    pub fn is_wall(&self) -> bool {
        match self {
            Tile::Air => false,
            Tile::Solid => true,
            Tile::SpikesLeft => false,
            Tile::SpikesRight => false,
            Tile::SpikesUp => false,
            Tile::SpikesDown => false,
            Tile::SpawnPoint => false,
            Tile::GoalLeft => false,
            Tile::GoalRight => false,
            Tile::GoalUp => false,
            Tile::GoalDown => false,
            Tile::SpikeAllSides => false,
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

    fn color(&self) -> Color {
        match self {
            Tile::Air => Color::WHITE,
            Tile::Solid => Color::BLACK,
            Tile::SpikesLeft => Color::BLACK,
            Tile::SpikesRight => Color::BLACK,
            Tile::SpikesUp => Color::BLACK,
            Tile::SpikesDown => Color::BLACK,
            Tile::SpawnPoint => Color::GREEN,
            Tile::GoalLeft => Color::ORANGE,
            Tile::GoalRight => Color::ORANGE,
            Tile::GoalUp => Color::ORANGE,
            Tile::GoalDown => Color::ORANGE,
            Tile::SpikeAllSides => Color::RED,
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

        let vertices = TilemapRenderer::get_tilemap_vertices(tilemap);

        let size = vertices.len() * std::mem::size_of::<ColoredVertex>();
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tilemap_vertex_buffer"),
            size: size as _,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        vertex_buffer.slice(..).get_mapped_range_mut()[..size as usize]
            .copy_from_slice(bytemuck::cast_slice(&vertices));
        vertex_buffer.unmap();

        let render_pipeline =
            device.create_render_pipeline(&rendering::create_pipeline_descriptor(
                Some("tilemap_pipeline"),
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

    fn get_tilemap_vertices(tilemap: &Tilemap) -> Vec<ColoredVertex> {
        let mut vertices = Vec::with_capacity(5000);

        for y in 0..tilemap.height() {
            for x in 0..tilemap.width() {
                let tile = tilemap.get_tile(x, y);

                match tile {
                    Tile::Air
                    | Tile::Solid
                    | Tile::SpawnPoint
                    | Tile::GoalLeft
                    | Tile::GoalRight
                    | Tile::GoalUp
                    | Tile::GoalDown => TilemapRenderer::append_vertices_solid(
                        tile,
                        &mut vertices,
                        FVec2::new(x as f32, y as f32),
                    ),
                    Tile::SpikesLeft => TilemapRenderer::append_vertices_spikes(
                        tile,
                        &mut vertices,
                        FVec2::new(x as f32, y as f32),
                        true,
                        false,
                        false,
                        false,
                    ),
                    Tile::SpikesRight => TilemapRenderer::append_vertices_spikes(
                        tile,
                        &mut vertices,
                        FVec2::new(x as f32, y as f32),
                        false,
                        true,
                        false,
                        false,
                    ),
                    Tile::SpikesUp => TilemapRenderer::append_vertices_spikes(
                        tile,
                        &mut vertices,
                        FVec2::new(x as f32, y as f32),
                        false,
                        false,
                        true,
                        false,
                    ),
                    Tile::SpikesDown => TilemapRenderer::append_vertices_spikes(
                        tile,
                        &mut vertices,
                        FVec2::new(x as f32, y as f32),
                        false,
                        false,
                        false,
                        true,
                    ),
                    Tile::SpikeAllSides => TilemapRenderer::append_vertices_spikes(
                        tile,
                        &mut vertices,
                        FVec2::new(x as f32, y as f32),
                        true,
                        true,
                        true,
                        true,
                    ),
                }
            }
        }

        vertices
    }

    pub fn append_vertices_solid(tile: Tile, vertices: &mut Vec<ColoredVertex>, pos: FVec2) {
        TilemapRenderer::append_rectangle(
            vertices,
            Bounds::new(pos, pos + FVec2::new(1.0, 1.0)),
            tile.color(),
        );
    }

    fn append_rectangle(vertices: &mut Vec<ColoredVertex>, bounds: Bounds, color: Color) {
        vertices.push(ColoredVertex::new(
            FVec2::new(bounds.min.x, bounds.max.y),
            color,
        ));
        vertices.push(ColoredVertex::new(
            FVec2::new(bounds.min.x, bounds.min.y),
            color,
        ));
        vertices.push(ColoredVertex::new(
            FVec2::new(bounds.max.x, bounds.max.y),
            color,
        ));
        vertices.push(ColoredVertex::new(
            FVec2::new(bounds.max.x, bounds.max.y),
            color,
        ));
        vertices.push(ColoredVertex::new(
            FVec2::new(bounds.min.x, bounds.min.y),
            color,
        ));
        vertices.push(ColoredVertex::new(
            FVec2::new(bounds.max.x, bounds.min.y),
            color,
        ));
    }

    fn append_vertices_spikes(
        tile: Tile,
        vertices: &mut Vec<ColoredVertex>,
        pos: FVec2,
        left: bool,
        right: bool,
        up: bool,
        down: bool,
    ) {
        TilemapRenderer::append_rectangle(
            vertices,
            Bounds::new(pos, pos + FVec2::new(1.0, 1.0)),
            Color::WHITE,
        );
        TilemapRenderer::append_spike(vertices, pos, left, right, up, down, tile.color());
    }

    /// Dynamically build spike vertices based on directions where spikes are enabled
    fn append_spike(
        vertices: &mut Vec<ColoredVertex>,
        pos: FVec2,
        left: bool,
        right: bool,
        up: bool,
        down: bool,
        color: Color,
    ) {
        // Can't use closures instead of macros here since both functions would require a mutable reference to `vertices`
        macro_rules! triangle {
            ($x0:expr, $y0:expr, $x1:expr, $y1: expr, $x2:expr, $y2: expr) => {
                vertices.push(ColoredVertex::new(
                    FVec2::new(pos.x + $x0, pos.y + $y0),
                    color,
                ));
                vertices.push(ColoredVertex::new(
                    FVec2::new(pos.x + $x1, pos.y + $y1),
                    color,
                ));
                vertices.push(ColoredVertex::new(
                    FVec2::new(pos.x + $x2, pos.y + $y2),
                    color,
                ));
            };
        }

        macro_rules! rect {
            ($x:expr, $y:expr, $w:expr, $h:expr) => {
                TilemapRenderer::append_rectangle(
                    vertices,
                    Bounds::new(
                        FVec2::new(pos.x + $x, pos.y + $y),
                        FVec2::new(pos.x + $x + $w, pos.y + $y + $h),
                    ),
                    color,
                );
            };
        }

        const S: f32 = 0.1;
        const SS: f32 = 0.7;

        if left && !up {
            triangle!(0.5 - S, 0.0, 0.0, 0.25, 0.5 - S, 0.5);
            rect!(0.5 - S, 0.0, S, 0.5);
        } else if !left && up {
            triangle!(0.0, 0.5 - S, 0.25, 0.0, 0.5, 0.5 - S);
            rect!(0.0, 0.5 - S, 0.5, S);
        } else if left && up {
            triangle!(0.0, 0.0, 0.0 + SS, 0.5 - S, 0.5 - S, SS);
        } else {
            rect!(0.0, 0.0, 0.5, 0.5);
        }

        if right && !up {
            triangle!(0.5 + S, 0.0, 1.0, 0.25, 0.5 + S, 0.5);
            rect!(0.5, 0.0, S, 0.5);
        } else if !right && up {
            triangle!(0.5, 0.5 - S, 0.75, 0.0, 1.0, 0.5 - S);
            rect!(0.5, 0.5 - S, 0.5, S);
        } else if right && up {
            triangle!(1.0, 0.0, 1.0 - SS, 0.5 - S, 0.5 + S, SS);
        } else {
            rect!(0.5, 0.0, 0.5, 0.5);
        }

        if left && !down {
            triangle!(0.5 - S, 0.5, 0.0, 0.75, 0.5 - S, 1.0);
            rect!(0.5 - S, 0.5, S, 0.5);
        } else if !left && down {
            triangle!(0.0, 0.5 + S, 0.25, 1.0, 0.5, 0.5 + S);
            rect!(0.0, 0.5, 0.5, S);
        } else if left && down {
            triangle!(0.0, 1.0, 0.5 - S, 1.0 - SS, SS, 0.5 + S);
        } else {
            rect!(0.0, 0.5, 0.5, 0.5);
        }

        if right && !down {
            triangle!(0.5 + S, 0.5, 1.0, 0.75, 0.5 + S, 1.0);
            rect!(0.5, 0.5, S, 0.5);
        } else if !right && down {
            triangle!(0.5, 0.5 + S, 0.75, 1.0, 1.0, 0.5 + S);
            rect!(0.5, 0.5, 0.5, S);
        } else if right && down {
            triangle!(1.0, 1.0, 0.5 - S, SS, 0.5 + S, 1.0 - SS);
        } else {
            rect!(0.5, 0.5, 0.5, 0.5);
        }
    }

    pub fn draw(&mut self, context: &mut DrawContext, state: &DrawState, world_type: WorldType) {
        let uniforms = TilemapUniforms {
            view_matrix: state.view_matrix,
            invert_colors: if world_type == WorldType::Dark { 1 } else { 0 },
            ..bytemuck::Zeroable::zeroed()
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
                        load: wgpu::LoadOp::Clear(if world_type == WorldType::Dark {
                            wgpu::Color::WHITE
                        } else {
                            wgpu::Color::BLACK
                        }),
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
    invert_colors: i32,
    padding: [i8; 12],
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
