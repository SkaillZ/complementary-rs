use serde::Deserialize;

use crate::{
    game::{ObjectTickState, WorldType},
    rendering::DrawState,
    window::DrawContext, math::FVec2,
};

use super::{Object, Tickable};

#[derive(Debug, Deserialize)]
pub struct WindData {}

pub type WindObject = Object<WindData, ()>;

impl WindObject {
    pub fn new(position: FVec2, data: WindData) -> Self {
        Self { position, data, state: () }
    }
}

impl Tickable for WindObject {
    fn tick(&mut self, state: &mut ObjectTickState) {
    }
}

#[derive(Debug)]
pub struct WindRenderer {}

impl WindRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {}
    }

    pub fn draw(
        &mut self,
        objects: &Vec<WindObject>,
        context: &mut DrawContext,
        state: &DrawState,
        world_type: WorldType,
    ) {
    }
}
