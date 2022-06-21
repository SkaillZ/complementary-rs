use std::time::{Duration, SystemTime};

use crate::{
    imgui_helpers::{ImGui, ImGuiSettings},
    input::{ButtonType, Input},
    math::FVec3,
    player::Player,
    rendering::DrawState,
    tilemap::{Tile, Tilemap, TilemapRenderer},
    window::DrawContext,
};
use cgmath::Zero;
use complementary_macros::ImGui;
use rand::Rng;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};

pub struct Game {
    rng: Xoshiro256PlusPlus,
    player: Player,
    tilemap: Tilemap,

    draw_state: DrawState,
    tilemap_renderer: TilemapRenderer,
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

        //let mut tilemap = Tilemap::default();
        let tilemap = Tilemap::load_from_file(
            "/Users/rene/repos/complementary/assets/maps/map001_intro_SWITCH.cmtm",
        )
        .expect("Failed to load first level");
        Game {
            rng: Xoshiro256PlusPlus::seed_from_u64(seed),
            player: Player::new(device),
            tilemap_renderer: TilemapRenderer::new(device, &tilemap),
            tilemap,

            draw_state: DrawState::new(),
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

    pub fn draw(&mut self, context: &mut DrawContext) {
        self.draw_state.update_view_matrix(
            context.window_width as f32,
            context.window_height as f32,
            self.tilemap.width() as f32,
            self.tilemap.height() as f32,
        );

        self.tilemap_renderer.draw(context, &self.draw_state);
        self.player.draw(context, &self.draw_state);
    }
}
