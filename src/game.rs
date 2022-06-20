use std::time::{Duration, SystemTime};

use crate::{
    imgui_helpers::{ImGui, ImGuiSettings},
    input::Input,
    math::FVec3,
    player::Player,
};
use cgmath::Zero;
use complementary_macros::ImGui;
use rand::Rng;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};

pub struct Game {
    rng: Xoshiro256PlusPlus,
    player: Player,
}

impl Game {
    // Tick 100 times per second
    pub const TICK_DURATION: Duration = Duration::new(0, 10000000);
    // Skip 5 frames max. between rendering
    pub const MAX_TICKS_PER_FRAME: i32 = 5;

    pub fn new(device: &wgpu::Device) -> Self {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::default())
            .as_secs();
        Game {
            rng: Xoshiro256PlusPlus::seed_from_u64(seed),
            player: Player::new(device),
        }
    }

    pub fn draw_gui(&mut self, gui: &imgui::Ui, input: &mut Input) {
        let _token = match imgui::Window::new("DevGUI")
            .size([400.0, 250.0], imgui::Condition::FirstUseEver)
            .begin(&gui)
        {
            Some(token) => token,
            None => return,
        };

        input.draw_gui("Input", gui);

        //self.player.draw_gui_with_settings("Player", gui, &ImGuiSettings::new().read_only())
    }

    pub fn tick(&mut self, input: &Input) {
        self.player.tick(input);
    }

    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::TextureView) {
        self.player.draw(encoder, output);
    }
}
