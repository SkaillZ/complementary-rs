use cgmath::Zero;
use complementary_macros::ImGui;
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array,
};

use crate::{
    imgui_helpers::ImGui,
    input::Input,
    math::{FVec2, FVec3},
    rendering::{create_pipeline_descriptor, Vertex},
};

pub type AbilityPair = (Ability, Ability);

pub enum Ability {
    None,
    DoubleJump,
    Glider,
    Dash,
    WallJump,
}

//#[derive(ImGui)]
pub struct Player {
    position: FVec2,
    velocity: FVec2,
    acceleration: FVec2,
    // abilities: AbilityPair
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
}

impl Player {
    pub const SIZE: FVec2 = FVec2::new(0.8, 0.8);

    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(&include_wgsl!("shaders/player.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[],
            label: Some("player_bind_group_layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[],
            label: Some("player_bind_group"),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            label: Some("player_pipeline_layout"),
            push_constant_ranges: &[],
        });

        let vertices = [
            Vertex::new(-Player::SIZE.x * 0.5, Player::SIZE.y),
            Vertex::new(-Player::SIZE.x * 0.5, 0.0),
            Vertex::new(Player::SIZE.x * 0.5, Player::SIZE.y),
            Vertex::new(Player::SIZE.x * 0.5, Player::SIZE.y),
            Vertex::new(-Player::SIZE.x * 0.5, 0.0),
            Vertex::new(Player::SIZE.x * 0.5, 0.0),
        ];

        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("player_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let render_pipeline = device.create_render_pipeline(&create_pipeline_descriptor(
            Some("player_pipeline"),
            &shader,
            Some(&pipeline_layout),
            &[Vertex::layout()],
        ));

        Player {
            position: FVec2::zero(),
            velocity: FVec2::zero(),
            acceleration: FVec2::zero(),

            buffer,
            bind_group,
            render_pipeline,
        }
    }

    pub fn tick(&mut self, input: &Input) {}

    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::TextureView) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            label: None,
        });
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_vertex_buffer(0, self.buffer.slice(..));
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.draw(0..6, 0..1);
    }
}
