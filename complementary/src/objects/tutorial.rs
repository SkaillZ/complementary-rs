use serde::Deserialize;

use crate::{
    game::{ObjectTickState, WorldType},
    rendering::DrawState,
    window::DrawContext, math::FVec2,
};

use super::{Object, Tickable};

#[derive(Debug, Deserialize)]
pub struct TutorialData {}

pub type TutorialObject = Object<TutorialData, ()>;

impl TutorialObject {
    pub fn new(position: FVec2, data: TutorialData) -> Self {
        Self { position, data, state: () }
    }
}

impl Tickable for TutorialObject {
    fn tick(&mut self, state: &mut ObjectTickState) {
    }
}

#[derive(Debug)]
pub struct TutorialRenderer {}

impl TutorialRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {}
    }

    pub fn draw(
        &mut self,
        objects: &Vec<TutorialObject>,
        context: &mut DrawContext,
        state: &DrawState,
        world_type: WorldType,
    ) {
    }
}
