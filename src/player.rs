use cgmath::Zero;
use complementary_macros::{ImGui};
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, BufferDescriptor,
};

use crate::{
    imgui_helpers::ImGui,
    input::{ButtonType, Input},
    math::{FMat4, FVec2, FVec3},
    rendering::{
        create_pipeline_descriptor, create_vertex_buffer, DrawState, UniformBuffer, Vertex,
    },
    window::DrawContext,
};

pub type AbilityPair = (Ability, Ability);

pub enum Ability {
    None,
    DoubleJump,
    Glider,
    Dash,
    WallJump,
}

#[derive(ImGui)]
pub struct Player {
    position: FVec2,
    velocity: FVec2,
    acceleration: FVec2,

    #[gui_ignore]
    abilities: AbilityPair,

    #[gui_ignore]
    render_state: PlayerRenderState,
}

pub struct PlayerRenderState {
    buffer: wgpu::Buffer,
    uniform_buffer: UniformBuffer<PlayerUniforms>,
    render_pipeline: wgpu::RenderPipeline,
}

impl Player {
    pub const SIZE: FVec2 = FVec2::new(0.8, 0.8);

    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = UniformBuffer::new(device, "player_uniforms");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniform_buffer.bind_group_layout()],
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

        let buffer = create_vertex_buffer(device, Some("player_vertex_buffer"), &vertices);

        let render_pipeline = device.create_render_pipeline(&create_pipeline_descriptor(
            Some("player_pipeline"),
            &device.create_shader_module(&include_wgsl!("shaders/player.wgsl")),
            Some(&pipeline_layout),
            &[Vertex::layout()],
        ));

        Player {
            position: FVec2::zero(),
            velocity: FVec2::zero(),
            acceleration: FVec2::zero(),

            abilities: (Ability::None, Ability::None),
            render_state: PlayerRenderState {
                buffer,
                uniform_buffer,
                render_pipeline,
            },
        }
    }

    pub fn tick(&mut self, input: &Input) {
        if input.get_button(ButtonType::Left).pressed() {
            self.position.x -= 0.1;
        }
        if input.get_button(ButtonType::Right).pressed() {
            self.position.x += 0.1;
        }
        if input.get_button(ButtonType::Up).pressed() {
            self.position.y += 0.1;
        }
        if input.get_button(ButtonType::Down).pressed() {
            self.position.y -= 0.1;
        }
    }

    pub fn draw(&mut self, context: &mut DrawContext, state: &DrawState) {
        let model_matrix =
            FMat4::from_translation(FVec3::new(self.position.x, self.position.y, 0.0));

        let uniforms = PlayerUniforms {
            view_matrix: state.view_matrix,
            model_matrix,
        };
        self.render_state
            .uniform_buffer
            .write_with_queue(context.queue, uniforms);

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
                label: None,
            });
        rpass.set_pipeline(&self.render_state.render_pipeline);
        rpass.set_vertex_buffer(0, self.render_state.buffer.slice(..));
        rpass.set_bind_group(0, &self.render_state.uniform_buffer.bind_group(), &[]);
        rpass.draw(0..6, 0..1);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PlayerUniforms {
    view_matrix: FMat4,
    model_matrix: FMat4,
}
