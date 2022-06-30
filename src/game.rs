use std::time::{Duration, SystemTime};

use crate::{
    imgui_helpers::{ImGui, ImGuiSettings},
    input::{ButtonType, Input},
    level::{self, Level, LevelLoadError},
    math::FVec3,
    player::Player,
    rendering::DrawState,
    tilemap::{Tile, Tilemap, TilemapRenderer},
    window::DrawContext,
};
use cgmath::Zero;
use complementary_macros::ImGui;
use log::error;
use rand::Rng;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};

pub struct Game {
    rng: Xoshiro256PlusPlus,
    player: Player,
    level: Level,

    draw_state: DrawState,
    tilemap_renderer: TilemapRenderer,
}

impl Game {
    // Tick 100 times per second
    pub const TICK_DURATION: Duration = Duration::new(0, 10000000);
    // Skip 5 frames max. between rendering
    pub const MAX_TICKS_PER_FRAME: i32 = 5;

    pub fn new(device: &wgpu::Device) -> Result<Self, LevelLoadError> {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::default())
            .as_secs();

        let level = Level::default();

        let mut game = Game {
            rng: Xoshiro256PlusPlus::seed_from_u64(seed),
            player: Player::new(device),

            tilemap_renderer: TilemapRenderer::new(device, &level.tilemap),
            level,
            draw_state: DrawState::new(),
        };

        game.load_level(device, "title")?;
        Ok(game)
    }

    pub fn draw_gui(&mut self, gui: &imgui::Ui, input: &mut Input, device: &wgpu::Device) {
        let _token = match imgui::Window::new("DevGUI")
            .size([400.0, 250.0], imgui::Condition::FirstUseEver)
            .begin(&gui)
        {
            Some(token) => token,
            None => return,
        };

        lazy_static::lazy_static! {
            static ref ALL_LEVELS: Vec<String> = level::get_all_levels().expect("Failed to load levels");
        }

        if gui.collapsing_header("Levels", imgui::TreeNodeFlags::empty()) {
            gui.indent();
            for level_name in &*ALL_LEVELS {
                if gui.button(level_name) {
                    if let Err(err) = self.load_level(device, level_name) {
                        error!("{err}");
                    }
                }
            }
            gui.unindent();
        }
        input.draw_gui("Input", gui);
        self.player.draw_gui("Player", gui);
    }

    pub fn tick(&mut self, input: &Input) {
        self.player.tick(input);
    }

    pub fn draw(&mut self, context: &mut DrawContext) {
        self.draw_state.update_view_matrix(
            context.window_width as f32,
            context.window_height as f32,
            self.level.tilemap.width() as f32,
            self.level.tilemap.height() as f32,
        );

        self.tilemap_renderer.draw(context, &self.draw_state);
        self.player.draw(context, &self.draw_state);
    }

    pub fn load_level(&mut self, device: &wgpu::Device, name: &str) -> Result<(), LevelLoadError> {
        self.level = Level::load(name)?;
        self.tilemap_renderer = TilemapRenderer::new(device, &self.level.tilemap);
        Ok(())
    }
}
