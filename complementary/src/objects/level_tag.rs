use serde::Deserialize;

use crate::{
    game::{ObjectTickState, WorldType},
    math::FVec2,
    player::AbilityPair,
    rendering::DrawState,
    window::DrawContext,
};

use super::{Object, Tickable};

#[derive(Debug, Deserialize)]
pub struct LevelTagData {}

pub type LevelTagObject = Object<LevelTagData, ()>;

impl LevelTagObject {
    pub fn new(position: FVec2, data: LevelTagData) -> Self {
        Self { position, data, state: () }
    }
}

impl Tickable for LevelTagObject {
    fn tick(&mut self, _state: &mut ObjectTickState) {
    }
}

#[derive(Debug)]
pub struct LevelTagRenderer {}

impl LevelTagRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {}
    }

    pub fn draw(
        &mut self,
        objects: &Vec<LevelTagObject>,
        context: &mut DrawContext,
        state: &DrawState,
        world_type: WorldType,
    ) {
    }
}
