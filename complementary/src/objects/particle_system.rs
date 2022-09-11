use serde::Deserialize;

use crate::{
    game::{ObjectTickState, WorldType},
    rendering::DrawState,
    window::DrawContext, math::{FVec2, Color},
};

use super::{Object, Tickable};

#[derive(Debug, Deserialize)]
pub struct ParticleSystemData {
    duration: i32,
    #[serde(rename = "type")]
    particle_type: ParticleType,
    min_emission_interval: i32,
    max_emission_interval: i32,
    min_emission_rate: i32,
    max_emission_rate: i32,

    min_start_velocity: FVec2,
    max_start_velocity: FVec2,
    gravity: f32,
    max_life_time: i32,
    start_color: Color,
    end_color: Color,
    start_size: f32,
    end_size: f32,
    follow_player: bool,
    play_on_spawn: bool,
    destroy_on_end: bool,
    enable_collision: bool,
    clamp_position_in_bounds: bool,

    emission_type: ParticleEmissionType,
    attract_speed: f32,
    layer: ParticleLayer,
    auto_invert_color: bool,
    out_of_box_lifetime_loss: i32,
    clamp_box_size: FVec2,
    symmetrical: bool,
}

#[derive(Debug, Deserialize)]
enum ParticleType {
    Triangle,
    Square,
    Diamond,
}

#[derive(Debug, Deserialize)]
enum ParticleLayer {
    BehindTilemap,
    OverTilemap,
}

#[derive(Debug, Deserialize)]
enum ParticleEmissionType {
    Center,
    BoxEdge(FVec2),
    Box(FVec2),
    Wind,
    BoxEdgeSpiky(FVec2),
}

struct Particle {
    position: FVec2,
    velocity: FVec2,
    lifetime: i32
}

pub struct ParticleSystemState {
    particles: Vec<Particle>
}

pub type ParticleSystemObject = Object<ParticleSystemData, ParticleSystemState>;

impl ParticleSystemObject {
    pub fn new(position: FVec2, data: ParticleSystemData) -> Self {
        Self { position, data, state: ParticleSystemState { particles: Vec::with_capacity(128) } }
    }
}

impl Tickable for ParticleSystemObject {
    fn tick(&mut self, state: &mut ObjectTickState) {
    }
}

struct ParticleInstance {
    color: Color,
    position: FVec2,
}

#[derive(Debug)]
pub struct ParticleSystemRenderer {}

impl ParticleSystemRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {}
    }

    pub fn draw(
        &mut self,
        objects: &Vec<ParticleSystemObject>,
        context: &mut DrawContext,
        state: &DrawState,
        world_type: WorldType,
    ) {
    }
}
