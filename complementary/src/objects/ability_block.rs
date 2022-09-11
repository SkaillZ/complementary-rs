use bytemuck::Zeroable;
use serde::Deserialize;
use wgpu::{include_wgsl, vertex_attr_array};

use crate::{
    game::{ObjectTickState, WorldType},
    math::{FVec2, FMat4, Color, Direction},
    player::{AbilityPair, Player},
    rendering::{DrawState, UniformBuffer, SQUARE_VERTICES, create_vertex_buffer, create_pipeline_descriptor, Vertex, create_instance_buffer},
    window::DrawContext, level::LevelState,
};

use super::{Object, Tickable, PositionalWithSize, Collidable};

#[derive(Debug, Deserialize)]
pub struct AbilityBlockData {
    size: FVec2,
    abilities: AbilityPair,
}

pub type AbilityBlockObject = Object<AbilityBlockData, ()>;

impl AbilityBlockObject {
    pub fn new(position: FVec2, data: AbilityBlockData) -> Self {
        Self { position, data, state: () }
    }
}

impl Tickable for AbilityBlockObject {
    fn tick(&mut self, state: &mut ObjectTickState) {
    }
}

impl PositionalWithSize for AbilityBlockObject {
    fn size(&self) -> FVec2 {
        self.data.size
    }
}

impl Collidable for AbilityBlockObject {
    fn on_directional_collision(&mut self, player: &mut Player, _level_state: &mut LevelState, _direction: Direction) {
        player.set_abilities(self.data.abilities)
    }
}

pub struct AbilityBlockRenderer {
    uniform_buffer: UniformBuffer<DrawState>,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct AbilityBlockInstance {
    color: Color,
    position: FVec2,
    size: FVec2,
}

impl AbilityBlockInstance {
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

impl AbilityBlockRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = UniformBuffer::new(device, "ability_block_uniforms");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniform_buffer.bind_group_layout()],
            label: Some("ability_block_pipeline_layout"),
            push_constant_ranges: &[],
        });

        let vertex_buffer = create_vertex_buffer(device, Some("ability_block_vertex_buffer"),
         &SQUARE_VERTICES);
        let instance_buffer = create_instance_buffer::<AbilityBlockInstance>(device, Some("ability_block_instance_buffer"),
        AbilityBlockInstance::MAX_INSTANCE_COUNT);

        let render_pipeline = device.create_render_pipeline(&create_pipeline_descriptor(
            Some("ability_block_pipeline"),
            &device.create_shader_module(&include_wgsl!("../shaders/ability_block.wgsl")),
            Some(&pipeline_layout),
            &[Vertex::layout(), AbilityBlockInstance::layout()],
        ));

        Self { uniform_buffer, vertex_buffer, instance_buffer, render_pipeline }
    }

    pub fn draw(
        &mut self,
        objects: &Vec<AbilityBlockObject>,
        context: &mut DrawContext,
        state: &DrawState,
        world_type: WorldType,
    ) {
        let instances: Vec<_> = objects.iter().map(|obj| AbilityBlockInstance {
            color: obj.data.abilities.current(world_type).color(),
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
