use std::fmt;

use cgmath::{ElementWise, Zero};
use complementary_macros::ImGui;
use log::debug;
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, BufferDescriptor,
};

use crate::{
    game::{Game, TickState, WorldType},
    imgui_helpers::ImGui,
    input::{ButtonType, Input},
    level::Level,
    math::{Bounds, Color, Direction, FMat4, FVec2, FVec3},
    rendering::{
        create_pipeline_descriptor, create_vertex_buffer, DrawState, UniformBuffer, Vertex,
    },
    tilemap::{Tile, Tilemap},
    window::DrawContext,
};

#[derive(ImGui)]
pub struct Player {
    dead: bool,
    position: FVec2,
    velocity: FVec2,
    acceleration: FVec2,

    /// Used to apply velocity from platforms etc.
    base_velocity: FVec2,

    /// Jump buffering (see https://twitter.com/maddythorson/status/1238338575545978880)
    current_jump_buffer_ticks: i32,
    /// Coyote time (see https://twitter.com/MaddyThorson/status/1238338574220546049)
    /// The value is `MAX_COYOTE_TIME` if we're grounded or value decreasing from `MAX_COYOTE_TIME`
    /// to zero if we're in the air. Called `fakeGrounded` in C++ version
    ground_coyote_time: i32,
    /// Decreasing timer which applies a force each frame after a jump for `MAX_JUMP_TICKS` frames
    /// as long as the player keeps holding the Jump button. This allows precise control over the jump height.
    jump_ticks: i32,

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

    pub const MOVE_SPEED: f32 = 0.04;
    pub const MOVE_SPEED_EXPONENT: f32 = 5.0;
    pub const GRAVITY: FVec2 = FVec2::new(0.0, 0.0275);
    pub const DRAG: FVec2 = FVec2::new(0.7, 0.9);

    const INITIAL_JUMP_FORCE: FVec2 = FVec2::new(0.0, -0.3);
    const CONTINUOUS_JUMP_FORCE: FVec2 = FVec2::new(0.0, -0.1);
    const MAX_JUMP_TICKS: i32 = 40;
    const MAX_JUMP_BUFFER_TICKS: i32 = 6;
    const MAX_COYOTE_TIME: i32 = 5;
    const COLLISION_STEP: f32 = 0.0025;

    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = UniformBuffer::new(device, "player_uniforms");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniform_buffer.bind_group_layout()],
            label: Some("player_pipeline_layout"),
            push_constant_ranges: &[],
        });

        let vertices = [
            Vertex::new(0.0, Player::SIZE.y),
            Vertex::new(0.0, 0.0),
            Vertex::new(Player::SIZE.x, Player::SIZE.y),
            Vertex::new(Player::SIZE.x, Player::SIZE.y),
            Vertex::new(0.0, 0.0),
            Vertex::new(Player::SIZE.x, 0.0),
        ];

        let buffer = create_vertex_buffer(device, Some("player_vertex_buffer"), &vertices);

        let render_pipeline = device.create_render_pipeline(&create_pipeline_descriptor(
            Some("player_pipeline"),
            &device.create_shader_module(&include_wgsl!("shaders/player.wgsl")),
            Some(&pipeline_layout),
            &[Vertex::layout()],
        ));

        Player {
            position: FVec2::new(30.0, 30.0),
            velocity: FVec2::zero(),
            acceleration: FVec2::zero(),
            base_velocity: FVec2::zero(),
            dead: false,
            jump_ticks: 0,
            current_jump_buffer_ticks: 0,
            ground_coyote_time: 0,

            abilities: AbilityPair::default(),
            render_state: PlayerRenderState {
                buffer,
                uniform_buffer,
                render_pipeline,
            },
        }
    }

    pub fn tick(&mut self, state: &mut TickState) {
        let horizontal = state.input.get_button(ButtonType::Right).pressed() as i32 as f32
            - state.input.get_button(ButtonType::Left).pressed() as i32 as f32; // TODO: add input.get_horizontal()
        let right_force = horizontal.abs().powf(Player::MOVE_SPEED_EXPONENT)
            * Player::MOVE_SPEED
            * horizontal.signum();
        self.add_force(FVec2::new(right_force, 0.0));

        let collision_faces = self.handle_directional_collision(&state.level);

        self.add_force(Player::GRAVITY);

        if collision_faces[Direction::Down as usize] {
            self.ground_coyote_time = Player::MAX_COYOTE_TIME;
        }
        if self.ground_coyote_time > 0 {
            self.ground_coyote_time -= 1;
        }

        if state
            .input
            .get_button(ButtonType::Jump)
            .pressed_first_frame()
            && self.allowed_to_move()
        {
            self.current_jump_buffer_ticks = Player::MAX_JUMP_BUFFER_TICKS;
        }
        if self.current_jump_buffer_ticks > 0 {
            self.current_jump_buffer_ticks -= 1;
        }

        if self.current_jump_buffer_ticks > 0 && self.ground_coyote_time > 0 {
            self.add_force(Player::INITIAL_JUMP_FORCE);
            self.jump_ticks = Player::MAX_JUMP_TICKS;
            self.velocity.y = 0.0;
            self.ground_coyote_time = 0;
        }

        if !state.input.get_button(ButtonType::Jump).pressed() && self.allowed_to_move() {
            // Cancel the jump
            self.jump_ticks = 0;
        }

        if self.jump_ticks > 0 {
            // Add an additional force for some time as long as the player keeps holding the Jump button,
            // scaled by jump duration
            self.add_force(
                Player::CONTINUOUS_JUMP_FORCE
                    * (1.0 / 1.1_f32.powf((Player::MAX_JUMP_TICKS + 1 - self.jump_ticks) as f32)),
            );
            self.jump_ticks -= 1;
        }

        self.velocity += self.acceleration;
        self.velocity.mul_assign_element_wise(Player::DRAG);
        self.velocity += (FVec2::new(1.0, 1.0) - Player::DRAG).mul_element_wise(self.base_velocity);

        self.move_until_collision(&state.level.tilemap);

        self.acceleration = FVec2::zero();
        self.base_velocity = FVec2::zero();
    }

    pub fn draw(&mut self, context: &mut DrawContext, state: &DrawState, world_type: WorldType) {
        let model_matrix =
            FMat4::from_translation(FVec3::new(self.position.x, self.position.y, 0.0));

        let uniforms = PlayerUniforms {
            view_matrix: state.view_matrix,
            model_matrix,
            color: self.active_ability(world_type).color(),
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

    pub fn add_force(&mut self, force: FVec2) {
        self.acceleration += force;
    }

    /// Whether the player is colliding with the tile map or an object
    pub fn is_colliding(&self, tilemap: &Tilemap) -> bool {
        let bounds = self.get_bounds();
        if !tilemap.contains_bounds(bounds) {
            return true;
        }
        for y in bounds.min.y as i32..=bounds.max.y as i32 {
            for x in bounds.min.x as i32..=bounds.max.x as i32 {
                if tilemap.get_tile(x, y).is_solid() {
                    return true;
                }
            }
        }
        false
    }

    /// Get the bounding box of the player in world space
    pub fn get_bounds(&self) -> Bounds {
        Bounds::new(self.position, self.position + Player::SIZE)
    }

    /// Move the player in small steps, interrupting movement on collision
    fn move_until_collision(&mut self, tilemap: &Tilemap) {
        let mut energy = self.velocity;
        while energy.x != 0.0 || energy.y != 0.0 {
            // Move X component
            let old_x = self.position.x;
            if energy.x > Player::COLLISION_STEP {
                self.position.x += Player::COLLISION_STEP;
                energy.x -= Player::COLLISION_STEP;
            } else if energy.x < -Player::COLLISION_STEP {
                self.position.x -= Player::COLLISION_STEP;
                energy.x += Player::COLLISION_STEP;
            } else {
                self.position.x += energy.x;
                energy.x = 0.0;
            }
            if self.is_colliding(tilemap) {
                energy.x = 0.0;
                self.position.x = old_x;
                self.velocity.x = 0.0;
            }

            // Move Y component
            let old_y = self.position.y;
            if energy.y > Player::COLLISION_STEP {
                self.position.y += Player::COLLISION_STEP;
                energy.y -= Player::COLLISION_STEP;
            } else if energy.y < -Player::COLLISION_STEP {
                self.position.y -= Player::COLLISION_STEP;
                energy.y += Player::COLLISION_STEP;
            } else {
                self.position.y += energy.y;
                energy.y = 0.0;
            }
            if self.is_colliding(tilemap) {
                energy.y = 0.0;
                self.position.y = old_y;
                self.velocity.y = 0.0;
            }
        }
    }

    /// Check on which direction the player has collided with something and handle the collision
    /// Returns a boolean for each direction that indicates if a collision took place
    fn handle_directional_collision(&mut self, level: &Level) -> [bool; 4] {
        let mut collisions_by_direction = [false; 4];
        for (i, direction) in Direction::ALL.iter().enumerate() {
            // Pretend that we've moved slightly in the given direction
            let min = self.position + direction.as_vec().mul_element_wise(Player::COLLISION_STEP);
            let max = min + Player::SIZE;
            let bounds = Bounds::new(min, max);

            if !level.tilemap.contains_bounds(bounds) {
                collisions_by_direction[i] = true;
            }

            'outer: for y in bounds.min.y as i32..=bounds.max.y as i32 {
                for x in bounds.min.x as i32..=bounds.max.x as i32 {
                    let tile = level.tilemap.get_tile(x, y);
                    if tile.is_solid() {
                        collisions_by_direction[i] = true;
                        if matches!(
                            tile,
                            Tile::SpikeAllSides
                                | Tile::SpikesLeft
                                | Tile::SpikesRight
                                | Tile::SpikesUp
                                | Tile::SpikesDown
                        ) {
                            match tile.direction() {
                                Some(tile_dir) => {
                                    if *direction == tile_dir.inverse() {
                                        // Only kill if the direction of the spike is the inverse to the one we're testing
                                        self.kill();
                                        break 'outer;
                                    }
                                }
                                // The tile spike goes in all directions; always kill
                                None => {
                                    self.kill();
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
        }

        collisions_by_direction
    }

    pub fn kill(&mut self) {
        debug!("Player died");
        self.dead = true;
    }

    pub fn reset(&mut self, position: FVec2) {
        self.position = position;
        self.dead = false;
    }

    pub fn position(&self) -> FVec2 {
        self.position
    }

    pub fn set_position(&mut self, position: FVec2) {
        self.position = position;
    }

    pub fn dead(&self) -> bool {
        self.dead
    }

    pub fn allowed_to_move(&self) -> bool {
        true
    }

    pub fn active_ability(&self, world_type: WorldType) -> Ability {
        if world_type == WorldType::Light {
            self.abilities.0
        } else {
            self.abilities.1
        }
    }

    pub fn set_ability(&mut self, world_type: WorldType, ability: Ability) {
        if world_type == WorldType::Light {
            self.abilities.0 = ability;
        } else {
            self.abilities.1 = ability;
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PlayerUniforms {
    view_matrix: FMat4,
    model_matrix: FMat4,
    color: Color,
}

pub type AbilityPair = (Ability, Ability);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ability {
    None,
    DoubleJump,
    Glider,
    Dash,
    WallJump,
}

impl Default for Ability {
    fn default() -> Self {
        Ability::None
    }
}

impl Ability {
    pub fn color(self) -> Color {
        match self {
            Ability::None => Color::GRAY,
            Ability::DoubleJump => Color::new_solid(0.75, 0.0, 0.75),
            Ability::Glider => Color::new_solid(0.25, 1.0, 0.25),
            Ability::Dash => Color::new_solid(1.0, 0.65, 0.0),
            Ability::WallJump => Color::new_solid(0.0, 0.35, 1.0),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Ability::None => "None",
            Ability::DoubleJump => "Double Jump",
            Ability::Glider => "Glider",
            Ability::Dash => "Dash",
            Ability::WallJump => "Wall Jump",
        }
    }

    pub fn tutorial_text(self) -> Option<String> {
        unimplemented!();
    }

    pub fn cycle(self) -> Self {
        match self {
            Ability::None => Ability::DoubleJump,
            Ability::DoubleJump => Ability::Glider,
            Ability::Glider => Ability::Dash,
            Ability::Dash => Ability::WallJump,
            Ability::WallJump => Ability::None,
        }
    }
}

impl fmt::Display for Ability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
