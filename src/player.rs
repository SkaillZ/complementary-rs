use std::fmt;

use cgmath::{ElementWise, Zero, InnerSpace};
use complementary_macros::ImGui;
use log::debug;
use wgpu::include_wgsl;

use crate::{
    game::{TickState, WorldType},
    imgui_helpers::ImGui,
    input::ButtonType,
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

    #[gui_ignore]
    abilities: AbilityPair,

    /// Used to apply velocity from platforms etc.
    base_velocity: FVec2,

    /// Jump buffering (see https://twitter.com/maddythorson/status/1238338575545978880)
    jump_buffer_ticks: i32,
    /// Coyote time (see https://twitter.com/MaddyThorson/status/1238338574220546049)
    /// The value is `MAX_COYOTE_TIME` if we're grounded or value decreasing from `MAX_COYOTE_TIME`
    /// to zero if we're in the air. Called `fakeGrounded` in C++ version
    ground_coyote_time: i32,
    /// Decreasing timer which applies a force each frame after a jump for `MAX_JUMP_TICKS` frames
    /// as long as the player keeps holding the Jump button. This allows precise control over the jump height.
    jump_ticks: i32,

    /// Whether the player is allowed to jump in the air while they have the Double Jump
    can_jump_in_air: bool,
    dash_state: DashState,
    wall_jump_state: WallJumpState,

    #[gui_ignore]
    render_state: PlayerRenderState,
}

#[derive(ImGui)]
pub struct DashState {
    /// Decreasing timer which applies a force each frame after a jump for `MAX_DASH_TICKS` frames
    dash_ticks: i32,
    cooldown: i32,
    /// Set to `true` when either the ground was touched or a wall was collided while the wall jump is active
    useable: bool,

    #[gui_ignore]
    direction: Direction,
}

impl DashState {
    const MAX_DASH_TICKS: i32 = 24;
    const MAX_COOLDOWN: i32 = 24;
    const DASH_FORCE: f32 = 0.35;

    fn dash_ready(&self) -> bool {
        self.dash_ticks <= 0 && self.cooldown <= 0 && self.useable
    }

    fn is_dashing(&self) -> bool {
        self.dash_ticks > 0
    }

    fn decrease_counters(&mut self) {
        self.dash_ticks = 0.max(self.dash_ticks - 1);
        self.cooldown = 0.max(self.cooldown - 1);
    }
}

impl Default for DashState {
    fn default() -> Self {
        // Dash to the right by default
        Self {
            direction: Direction::Right,
            dash_ticks: 0,
            cooldown: 0,
            useable: true,
        }
    }
}

#[derive(ImGui, Default)]
pub struct WallJumpState {
    wall_jump_ticks: i32,
    cooldown: i32,
    #[gui_ignore]
    direction: Option<Direction>,

    move_left_cooldown: i32,
    move_right_cooldown: i32,

    /// Counts down from the frame when a wall was touched
    left_wall_collision_buffer: i32,
    right_wall_collision_buffer: i32,

    /// Set if moving left/right AND we're still in the range of one of the above buffers
    left_wall_input_buffer: i32,
    right_wall_input_buffer: i32,
}

impl WallJumpState {
    const INITIAL_FORCE: FVec2 = FVec2::new(0.5, -0.4);
    /// Applied in the same direction as `INITIAL_FORCE`
    const CONTINUOUS_FORCE_MAGNITUDE: f32 = 0.1;
    const MAX_WALL_JUMP_TICKS: i32 = 40;
    const WALL_STICK_Y_DRAG: f32 = 0.3;
    const MAX_COOLDOWN: i32 = 10;
    const MAX_COLLISION_BUFFER_TICKS: i32 = 5;
    const MAX_INPUT_BUFFER_TICKS: i32 = 7;
    /// The player can't move in the direction of the wall jump for this amount of ticks after a wall jump
    const MOVE_COOLDOWN: i32 = 15;

    fn wall_jump_ready(&self) -> bool {
        self.wall_jump_ticks <= 0 && self.cooldown <= 0 && (self.left_wall_input_buffer > 0 || self.right_wall_input_buffer > 0)
    }

    fn wall_jump_active(&self) -> bool {
        self.wall_jump_ticks > 0
    }

    fn decrease_counters(&mut self) {
        self.wall_jump_ticks = 0.max(self.wall_jump_ticks - 1);
        self.cooldown = 0.max(self.cooldown - 1);
        self.move_left_cooldown = 0.max(self.move_left_cooldown - 1);
        self.move_right_cooldown = 0.max(self.move_right_cooldown - 1);
        self.left_wall_collision_buffer = 0.max(self.left_wall_collision_buffer - 1);
        self.right_wall_collision_buffer = 0.max(self.right_wall_collision_buffer - 1);
        self.left_wall_input_buffer = 0.max(self.left_wall_input_buffer - 1);
        self.right_wall_input_buffer = 0.max(self.right_wall_input_buffer - 1);
    }

    fn reset_buffers(&mut self) {
        self.left_wall_collision_buffer = 0;
        self.right_wall_collision_buffer = 0;
        self.left_wall_input_buffer = 0;
        self.right_wall_input_buffer = 0;
    }

    fn initial_force_with_direction(&self) -> FVec2 {
        let direction = self.direction.unwrap_or(Direction::Right);
        assert!(matches!(direction, Direction::Left | Direction::Right), "Wall jump direction must be left or right");
        let mut force = WallJumpState::INITIAL_FORCE;
        force.x *= direction.as_vec().x;
        force
    }
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
    pub const GRAVITY_GLIDER: FVec2 = FVec2::new(0.0, 0.005);
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
            abilities: AbilityPair::default(),

            base_velocity: FVec2::zero(),
            dead: false,
            jump_ticks: 0,
            jump_buffer_ticks: 0,
            ground_coyote_time: 0,

            dash_state: DashState::default(),
            wall_jump_state: WallJumpState::default(),
            can_jump_in_air: false,

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
        if self.allowed_to_move() {
            let mut right_force = horizontal.abs().powf(Player::MOVE_SPEED_EXPONENT)
                * Player::MOVE_SPEED
                * horizontal.signum();

            if (right_force > 0.0 && self.wall_jump_state.move_right_cooldown > 0)
                || (right_force < 0.0 && self.wall_jump_state.move_left_cooldown > 0)
            {
                // Prevent moving against the direction where the player wall jumped from
                // for some ticks after a wall jump
                right_force = 0.0;
            }
            self.add_force(FVec2::new(right_force, 0.0));
        }

        
        self.apply_gravity(&state);
        
        let collision_faces = self.handle_directional_collision(&state.level);
        if collision_faces[Direction::Down as usize].is_some() {
            self.ground_coyote_time = Player::MAX_COYOTE_TIME;
            self.dash_state.useable = true;
            self.can_jump_in_air = true;
        }
        self.ground_coyote_time = 0.max(self.ground_coyote_time - 1);

        let left_wall_collision = matches!(collision_faces[Direction::Left as usize], Some(CollisionType::Wall));
        let right_wall_collision = matches!(collision_faces[Direction::Right as usize], Some(CollisionType::Wall));
        if left_wall_collision {
            self.wall_jump_state.left_wall_collision_buffer = WallJumpState::MAX_COLLISION_BUFFER_TICKS;
        }
        if right_wall_collision {
            self.wall_jump_state.right_wall_collision_buffer = WallJumpState::MAX_COLLISION_BUFFER_TICKS;
        }

        if state
            .input
            .get_button(ButtonType::Jump)
            .pressed_first_frame()
            && self.allowed_to_move()
        {
            self.jump_buffer_ticks = Player::MAX_JUMP_BUFFER_TICKS;
        }
        self.jump_buffer_ticks = 0.max(self.jump_buffer_ticks - 1);

        if self.allowed_to_move() {
            // Buffer directional inputs required for wall jumps, so that a slight delay after
            // holding the button registers as a wall jump
            if self.wall_jump_state.left_wall_collision_buffer > 0 && horizontal < 0.0 {
                self.wall_jump_state.left_wall_input_buffer = WallJumpState::MAX_INPUT_BUFFER_TICKS;
            } else if self.wall_jump_state.right_wall_collision_buffer > 0 && horizontal > 0.0 {
                self.wall_jump_state.right_wall_input_buffer =
                    WallJumpState::MAX_INPUT_BUFFER_TICKS;
            }
        }

        if self.jump_buffer_ticks > 0 {
            self.start_jumping(state);
        }

        self.wall_jump_state.decrease_counters();

        if self.wall_jump_state.wall_jump_active() {
            let normalized_direction = self.wall_jump_state.initial_force_with_direction().normalize();
            let force = normalized_direction * WallJumpState::CONTINUOUS_FORCE_MAGNITUDE *
            1.0 / 1.1_f32.powf(WallJumpState::MAX_WALL_JUMP_TICKS as f32 + 1.0 - self.wall_jump_state.wall_jump_ticks as f32);
            self.add_force(force);

            // Apply the direction if the wall jump to the dash too
            self.dash_state.direction = self.wall_jump_state.direction.unwrap_or(Direction::Right);
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

        // Set the dash direction based on the last horizontal input
        if !horizontal.is_zero() {
            self.dash_state.direction = if horizontal > 0.0 {
                Direction::Right
            } else {
                Direction::Left
            };
        }

        let mut drag = Player::DRAG;

        match self.active_ability(state.world_type) {
            Ability::Dash => self.tick_dash_active(state),
            Ability::WallJump => self.handle_wall_sticking(&mut drag, horizontal, left_wall_collision, right_wall_collision),
            _ => ()
        }

        self.dash_state.decrease_counters();

        if self.dash_state.is_dashing() {
            // The cosine here leads to a decrease of the dash velocity over time
            let dash_velocity = self.dash_state.direction.as_vec() * DashState::DASH_FORCE;
            self.velocity = dash_velocity
                * f32::cos(
                    std::f32::consts::PI
                        * 0.5
                        * (1.0
                            - self.dash_state.dash_ticks as f32 / DashState::MAX_DASH_TICKS as f32),
                );
        }

        self.velocity += self.acceleration;
        self.velocity.mul_assign_element_wise(drag);
        self.velocity += (FVec2::new(1.0, 1.0) - drag).mul_element_wise(self.base_velocity);

        self.move_until_collision(&state.level.tilemap);

        self.acceleration = FVec2::zero();
        self.base_velocity = FVec2::zero();
    }

    fn start_jumping(&mut self, state: &TickState) {
        if (self.grounded()
            || self.active_ability(state.world_type) == Ability::DoubleJump && self.can_jump_in_air)
            && !self.dash_state.is_dashing()
        {
            // Regular jump or double jump
            self.jump_buffer_ticks = 0;
            self.add_force(Player::INITIAL_JUMP_FORCE);
            self.jump_ticks = Player::MAX_JUMP_TICKS;
            self.velocity.y = 0.0;
            self.wall_jump_state.cooldown = WallJumpState::MAX_COOLDOWN;

            if !self.grounded() {
                self.can_jump_in_air = false;
            }
            self.ground_coyote_time = 0;
        } else if self.active_ability(state.world_type) == Ability::WallJump && self.wall_jump_state.wall_jump_ready() {
            // Wall jump
            self.wall_jump_state.direction = Some(if self.wall_jump_state.left_wall_input_buffer > 0 { Direction::Right } else { Direction::Left });
            debug!("Wall jump direction: {:?}", self.wall_jump_state.direction);
            let force = self.wall_jump_state.initial_force_with_direction();
            self.add_force(force);
            self.jump_buffer_ticks = 0;
            
            self.wall_jump_state.cooldown = WallJumpState::MAX_COOLDOWN;
            self.wall_jump_state.wall_jump_ticks = WallJumpState::MAX_WALL_JUMP_TICKS;
            self.wall_jump_state.reset_buffers();
            if self.wall_jump_state.direction == Some(Direction::Right) {
                self.wall_jump_state.move_right_cooldown = WallJumpState::MOVE_COOLDOWN;
            } else {
                self.wall_jump_state.move_left_cooldown = WallJumpState::MOVE_COOLDOWN;
            }
            self.reset_dash();
        }
    }

    fn tick_dash_active(&mut self, state: &TickState) {
        if (state.input.ability_button_pressed_first_frame())
            && self.allowed_to_move()
            && self.dash_state.dash_ready()
        {
            self.dash_state.dash_ticks = DashState::MAX_DASH_TICKS;
            self.dash_state.useable = false;
            self.dash_state.cooldown = DashState::MAX_DASH_TICKS + DashState::MAX_COOLDOWN;
            debug!("Dashing");
        }
    }

    fn handle_wall_sticking(&mut self, drag: &mut FVec2, horizontal: f32, left: bool, right: bool) {
        if self.velocity.y > 0.0 && ((left && horizontal < 0.0) || (right && horizontal > 0.0)) {
            drag.y *= WallJumpState::WALL_STICK_Y_DRAG;
        }
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
    /// Returns the type of collision that took place for each direction
    fn handle_directional_collision(&mut self, level: &Level) -> [Option<CollisionType>; 4] {
        let mut collisions_by_direction = [None; 4];
        for (i, direction) in Direction::ALL.iter().enumerate() {
            // Pretend that we've moved slightly in the given direction
            let min = self.position + direction.as_vec().mul_element_wise(Player::COLLISION_STEP);
            let max = min + Player::SIZE;
            let bounds = Bounds::new(min, max);

            if !level.tilemap.contains_bounds(bounds) {
                // Treat out of bounds as walls
                collisions_by_direction[i] = Some(CollisionType::Wall);
            }

            'outer: for y in bounds.min.y as i32..=bounds.max.y as i32 {
                for x in bounds.min.x as i32..=bounds.max.x as i32 {
                    let tile = level.tilemap.get_tile(x, y);
                    if tile.is_solid() {
                        collisions_by_direction[i] = Some(if tile.is_wall() {
                            CollisionType::Wall
                        } else {
                            CollisionType::Solid
                        });

                        // Handle collision with spikes
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

        self.velocity = FVec2::zero();
        self.acceleration = FVec2::zero();
        self.reset_dash();
        self.wall_jump_state = WallJumpState::default();
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

    /// Whether the player is considered to be "on the ground" (coyote time included!)
    pub fn grounded(&self) -> bool {
        self.ground_coyote_time > 0
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

    fn reset_dash(&mut self) {
        self.dash_state = DashState::default();
    }

    fn apply_gravity(&mut self, state: &TickState) {
        self.add_force(
            if self.active_ability(state.world_type) == Ability::Glider
                && state.input.ability_button_pressed()
                && self.velocity.y > 0.0
                && self.allowed_to_move()
            {
                Player::GRAVITY_GLIDER
            } else {
                Player::GRAVITY
            },
        );
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum CollisionType {
    Solid,
    Wall,
}
