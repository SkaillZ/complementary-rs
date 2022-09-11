use serde::Deserialize;
use wgpu::{vertex_attr_array, include_wgsl};

use crate::{
    game::{ObjectTickState, WorldType},
    rendering::{DrawState, UniformBuffer, SQUARE_VERTICES, create_vertex_buffer, create_instance_buffer, create_pipeline_descriptor, Vertex},
    window::DrawContext, math::{Color, FVec2, Bounds}, player::CollisionType,
};

use super::{Object, Tickable, PositionalWithSize, Collidable};

#[derive(Debug, Deserialize)]
pub struct DoorData {
    size: FVec2,
    group: i32,
}

#[derive(Debug, Deserialize)]
pub struct DoorState {
    key_collected_percentage: f32
}

pub type DoorObject = Object<DoorData, DoorState>;

impl DoorObject {
    pub fn new(position: FVec2, data: DoorData) -> Self {
        Self { position, data, state: DoorState { key_collected_percentage: 0.0 } }
    }
}

impl Tickable for DoorObject {
    fn tick(&mut self, state: &mut ObjectTickState) {
        self.state.key_collected_percentage = state.level_state.key_collected_percentage(self.data.group);
    }
}

impl PositionalWithSize for DoorObject {
    fn size(&self) -> FVec2 {
        self.data.size
    }
}

impl Collidable for DoorObject {
    fn collides_with(&self, other: &Bounds, _world_type: WorldType) -> Option<CollisionType> {
        if self.state.key_collected_percentage < 1.0 {
		    self.bounds().overlaps(other).then_some(CollisionType::Solid)
        } else {
            None
        }
	}
}

pub struct DoorRenderer {
    uniform_buffer: UniformBuffer<DrawState>,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DoorInstance {
    color: Color,
    position: FVec2,
    size: FVec2,
}

impl DoorInstance {
    const MAX_INSTANCE_COUNT: usize = 50;

    const ATTR: &'static [wgpu::VertexAttribute] = &vertex_attr_array![1 => Float32x4, 2 => Float32x2, 3 => Float32x2];

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::ATTR,
        }
    }
}

impl DoorRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = UniformBuffer::new(device, "door_uniforms");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniform_buffer.bind_group_layout()],
            label: Some("door_pipeline_layout"),
            push_constant_ranges: &[],
        });

        let vertex_buffer = create_vertex_buffer(device, Some("door_vertex_buffer"),
         &SQUARE_VERTICES);
        let instance_buffer = create_instance_buffer::<DoorInstance>(device, Some("door_instance_buffer"),
        DoorInstance::MAX_INSTANCE_COUNT);

        let render_pipeline = device.create_render_pipeline(&create_pipeline_descriptor(
            Some("door_pipeline"),
            &device.create_shader_module(&include_wgsl!("../shaders/door.wgsl")),
            Some(&pipeline_layout),
            &[Vertex::layout(), DoorInstance::layout()],
        ));

        Self { uniform_buffer, vertex_buffer, instance_buffer, render_pipeline }
    }

    pub fn draw(
        &mut self,
        objects: &Vec<DoorObject>,
        context: &mut DrawContext,
        state: &DrawState,
        world_type: WorldType,
    ) {
        let instances: Vec<_> = objects.iter().map(|obj| DoorInstance {
            color: match world_type {
                WorldType::Light => Color::DARK_GRAY,
                WorldType::Dark => Color::LIGHT_GRAY,
            }.with_alpha(1.0 - obj.state.key_collected_percentage),
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
                label: Some("door_rpass"),
            });
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_bind_group(0, &self.uniform_buffer.bind_group(), &[]);
        rpass.draw(0..6, 0..instances.len() as u32);
    }
}
