use serde::Deserialize;
use wgpu::{vertex_attr_array, include_wgsl};

use crate::{
    game::{ObjectTickState, WorldType},
    rendering::{DrawState, UniformBuffer, create_vertex_buffer, DIAMOND_VERTICES, create_instance_buffer, Vertex, create_pipeline_descriptor},
    window::DrawContext, math::{Color, FVec2, Bounds, Direction}, player::{CollisionType, Player}, level::LevelState,
};

use super::{Object, Tickable, PositionalWithSize, Collidable};

#[derive(Debug, Deserialize)]
pub struct KeyData {
    group: i32
}

#[derive(Debug, Deserialize)]
pub enum KeyState {
    Collectible,
    Collected { ticks: i32 }
}

pub type KeyObject = Object<KeyData, KeyState>;

impl KeyObject {
    pub fn new(position: FVec2, data: KeyData) -> Self {
        Self { position, data, state: KeyState::Collectible }
    }

    pub fn group(&self) -> i32 {
        self.data.group
    }

    fn alpha(&self) -> f32 {
        const ALPHA_ANIM_TICKS: i32 = 30;

        match self.state {
            KeyState::Collectible => 1.0,
            KeyState::Collected { ticks } => 1.0 - (ticks as f32 / ALPHA_ANIM_TICKS as f32),
        }
    }
}

impl Tickable for KeyObject {
    fn tick(&mut self, _state: &mut ObjectTickState) {
        match self.state {
            KeyState::Collected { ref mut ticks } => {
                *ticks += 1;
            },
            _ => ()
        }
    }
}

impl PositionalWithSize for KeyObject {
    fn size(&self) -> FVec2 {
        FVec2::new(1.0, 1.0)
    }
}

impl Collidable for KeyObject {
    fn collides_with(&self, other: &Bounds, _world_type: WorldType) -> Option<CollisionType> {
        self.bounds().overlaps(other).then_some(CollisionType::NonSolid)
    }

    fn on_directional_collision(&mut self, _player: &mut Player, level_state: &mut LevelState, _direction: Direction) {
        if matches!(self.state, KeyState::Collectible) {
            level_state.add_collected_key(self.group());
            self.state = KeyState::Collected { ticks: 0 }
        }
    }
}

pub struct KeyRenderer {
    uniform_buffer: UniformBuffer<DrawState>,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct KeyInstance {
    color: Color,
    position: FVec2,
}

impl KeyInstance {
    const MAX_INSTANCE_COUNT: usize = 50;

    const ATTR: &'static [wgpu::VertexAttribute] = &vertex_attr_array![1 => Float32x4, 2 => Float32x2];

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::ATTR,
        }
    }
}

impl KeyRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = UniformBuffer::new(device, "key_uniforms");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniform_buffer.bind_group_layout()],
            label: Some("key_pipeline_layout"),
            push_constant_ranges: &[],
        });

        let vertex_buffer = create_vertex_buffer(device, Some("key_vertex_buffer"),
         &DIAMOND_VERTICES);
        let instance_buffer = create_instance_buffer::<KeyInstance>(device, Some("key_instance_buffer"),
        KeyInstance::MAX_INSTANCE_COUNT);

        let render_pipeline = device.create_render_pipeline(&create_pipeline_descriptor(
            Some("key_pipeline"),
            &device.create_shader_module(&include_wgsl!("../shaders/key.wgsl")),
            Some(&pipeline_layout),
            &[Vertex::layout(), KeyInstance::layout()],
        ));

        Self { uniform_buffer, vertex_buffer, instance_buffer, render_pipeline }
    }

    pub fn draw(
        &mut self,
        objects: &Vec<KeyObject>,
        context: &mut DrawContext,
        state: &DrawState,
        world_type: WorldType,
    ) {
        let instances: Vec<_> = objects.iter().map(|obj| KeyInstance {
            color: match world_type {
                WorldType::Light => Color::DARK_GRAY,
                WorldType::Dark => Color::LIGHT_GRAY,
            }.with_alpha(obj.alpha()),
            position: obj.position,
        }).collect();

        self.uniform_buffer
            .write_with_queue(context.queue, state.clone());
        context.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));

        let mut rpass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &context.output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
                label: Some("key_rpass"),
            });
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_bind_group(0, &self.uniform_buffer.bind_group(), &[]);
        rpass.draw(0..6, 0..instances.len() as u32);
    }
}
