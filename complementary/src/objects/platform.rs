use std::mem;

use cgmath::InnerSpace;
use serde::Deserialize;
use wgpu::{vertex_attr_array, include_wgsl};

use crate::{
    game::{ObjectTickState, WorldType},
    math::{FVec2, FMat4, Color, Direction, Bounds},
    player::{AbilityPair, Player, CollisionType},
    rendering::{DrawState, UniformBuffer, Vertex, create_vertex_buffer, SQUARE_VERTICES, create_instance_buffer, create_pipeline_descriptor},
    window::DrawContext,
};

use super::{Object, Tickable, PositionalWithSize, Collidable};

#[derive(Debug, Deserialize)]
pub struct PlatformData {
    size: FVec2,
    goal: FVec2,
    speed: f32,
    spiky: (bool, bool, bool, bool),
    world_type: Option<WorldType>,
}

#[derive(Debug)]
pub struct PlatformState {
    current_goal: FVec2,
    next_goal: FVec2,
}

pub type PlatformObject = Object<PlatformData, PlatformState>;

impl PlatformObject {
    pub fn new(position: FVec2, data: PlatformData) -> Self {
        let state = PlatformState { current_goal: position + data.goal, next_goal: position };
        Self { position, data, state }
    }
}

impl Tickable for PlatformObject {
    fn tick(&mut self, state: &mut ObjectTickState) {
        let delta = self.state.current_goal - self.position;
        let distance = delta.magnitude2();
        if distance < 0.0005 {
            mem::swap(&mut self.state.current_goal, &mut self.state.next_goal);
        }
        if distance < self.data.speed {
            self.position = self.state.current_goal;
            mem::swap(&mut self.state.current_goal, &mut self.state.next_goal);
        } else {
            self.position += delta.normalize() * self.data.speed;
        }
        
        // TODO: force move player
    }
}

impl PositionalWithSize for PlatformObject {
    fn size(&self) -> FVec2 {
        self.data.size
    }
}

impl Collidable for PlatformObject {
    fn collides_with(&self, other: &Bounds, world_type: WorldType) -> Option<CollisionType> {
        if self.data.world_type == Some(world_type) || self.data.world_type == None {
            self.bounds().overlaps(other).then_some(CollisionType::Wall)
        } else {
            None
        }
    }
}

pub struct PlatformRenderer {
    uniform_buffer: UniformBuffer<DrawState>,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PlatformInstance {
    color: Color,
    position: FVec2,
    size: FVec2,
}

impl PlatformInstance {
    const MAX_INSTANCE_COUNT: usize = 100;

    const ATTR: &'static [wgpu::VertexAttribute] = &vertex_attr_array![1 => Float32x4, 2 => Float32x2, 3 => Float32x2];

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::ATTR,
        }
    }
}

impl PlatformRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = UniformBuffer::new(device, "platform_uniforms");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniform_buffer.bind_group_layout()],
            label: Some("platform_pipeline_layout"),
            push_constant_ranges: &[],
        });

        let vertex_buffer = create_vertex_buffer(device, Some("platform_vertex_buffer"),
         &SQUARE_VERTICES);
        let instance_buffer = create_instance_buffer::<PlatformInstance>(device, Some("platform_instance_buffer"),
        PlatformInstance::MAX_INSTANCE_COUNT);

        let render_pipeline = device.create_render_pipeline(&create_pipeline_descriptor(
            Some("ability_block_pipeline"),
            &device.create_shader_module(&include_wgsl!("../shaders/platform.wgsl")),
            Some(&pipeline_layout),
            &[Vertex::layout(), PlatformInstance::layout()],
        ));

        Self { uniform_buffer, vertex_buffer, instance_buffer, render_pipeline }
    }

    pub fn draw(
        &mut self,
        objects: &Vec<PlatformObject>,
        context: &mut DrawContext,
        state: &DrawState,
        world_type: WorldType,
    ) {
        let instances: Vec<_> = objects.iter().map(|obj| PlatformInstance {
            color: match obj.data.world_type {
                Some(ty) => {
                    if ty == world_type {
                        ty.foreground_color()
                    } else {
                        Color::TRANSPARENT
                    }
                },
                None => world_type.foreground_color(),
            },
            position: obj.position,
            size: obj.data.size,
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
                label: Some("ability_block_rpass"),
            });
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_bind_group(0, &self.uniform_buffer.bind_group(), &[]);
        rpass.draw(0..6, 0..instances.len() as u32);
    }
}
