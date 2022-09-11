use std::time::{Duration, SystemTime};

use crate::{
    imgui_helpers::ImGui,
    input::{ButtonType, Input},
    level::{self, Level, LevelLoadError, LevelState},
    objects::{ObjectSet, Tickable},
    player::Player,
    rendering::DrawState,
    tilemap::{Tilemap, TilemapRenderer},
    window::DrawContext, math::Color, audio,
};
use log::error;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};
use serde::Deserialize;

pub struct Game {
    rng: Xoshiro256PlusPlus,
    player: Player,
    level: Level,
    level_index: usize,
    world_type: WorldType,

    draw_state: DrawState,
}

pub struct PlayerTickState<'a> {
    pub input: &'a Input,
    pub tilemap: &'a mut Tilemap,
    pub objects: &'a mut ObjectSet,
    pub level_state: &'a mut LevelState,
    pub world_type: WorldType,
}

pub struct ObjectTickState<'a> {
    pub input: &'a Input,
    pub tilemap: &'a mut Tilemap,
    pub player: &'a mut Player,
    pub level_state: &'a mut LevelState,
    pub world_type: WorldType,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
pub enum WorldType {
    Light,
    Dark,
}

impl WorldType {
    pub fn inverse(self) -> Self {
        match self {
            WorldType::Light => WorldType::Dark,
            WorldType::Dark => WorldType::Light,
        }
    }

    pub fn foreground_color(self) -> Color {
        match self {
            WorldType::Light => Color::BLACK,
            WorldType::Dark => Color::WHITE,
        }
    }
}

lazy_static::lazy_static! {
    static ref ALL_LEVELS: Vec<String> = level::get_all_levels().expect("Failed to load levels");
    static ref MAIN_LEVELS: Vec<&'static String> = ALL_LEVELS.iter().filter(|level| level.starts_with("map")).collect();
}

impl Game {
    // Tick 100 times per second
    pub const TICK_DURATION: Duration = Duration::new(0, 10000000);
    // Skip 5 frames max. between rendering
    pub const MAX_TICKS_PER_FRAME: i32 = 5;

    pub fn new(device: &wgpu::Device) -> Result<Self, GameLoadError> {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::default())
            .as_secs();

        let mut game = Game {
            rng: Xoshiro256PlusPlus::seed_from_u64(seed),
            player: Player::new(device),
            world_type: WorldType::Light,
            level: Level::load(device, MAIN_LEVELS.first().expect("No levels loaded"))?,
            level_index: 0,
            draw_state: DrawState::new(),
        };

        game.spawn_player();
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

        if gui.button("Change ability") {
            self.player.set_ability(
                self.world_type,
                self.player.active_ability(self.world_type).cycle(),
            );
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

    pub fn tick(&mut self, input: &Input, device: &wgpu::Device) {
        if input.get_button(ButtonType::Switch).pressed_first_frame()
            || input
                .get_button(ButtonType::SwitchAndAbility)
                .pressed_first_frame()
        {
            if !self.player.is_colliding_with_solid_objects(&self.level.objects, self.world_type.inverse()) {
                // Only allow switching if the player is not colliding with an object
                // in the other world to avoid getting stuck
                self.world_type = self.world_type.inverse();
            }
        }

        audio::set_world(self.world_type);

        let mut state = PlayerTickState {
            input,
            tilemap: &mut self.level.tilemap,
            objects: &mut self.level.objects,
            level_state: &mut self.level.state,
            world_type: self.world_type,
        };

        self.player.tick(&mut state);

        let mut state = ObjectTickState {
            input,
            tilemap: &mut self.level.tilemap,
            player: &mut self.player,
            level_state: &mut self.level.state,
            world_type: self.world_type,
        };

        self.level.objects.tick(&mut state);

        if self.player.touched_goal() {
            if let Err(error) = self.next_level(device) {
                error!("Failed to load level: {}", error);
            }
        }
        if self.player.touched_goal() || self.player.dead() {
            let pos = self
                .level
                .tilemap
                .get_spawn_point()
                .unwrap_or(self.player.position());
            self.player.reset(pos);
        }
    }

    pub fn draw(&mut self, context: &mut DrawContext) {
        self.draw_state.update_view_matrix(
            context.window_width as f32,
            context.window_height as f32,
            self.level.tilemap.width() as f32,
            self.level.tilemap.height() as f32,
        );

        self.level
            .tilemap_renderer
            .draw(context, &self.draw_state, self.world_type);
        self.player.draw(context, &self.draw_state, self.world_type);
        self.level
            .objects
            .draw(context, &self.draw_state, self.world_type);
    }

    pub fn load_level(&mut self, device: &wgpu::Device, name: &str) -> Result<(), LevelLoadError> {
        let level = Level::load(device, name)?;
        self.level = level;
        self.spawn_player();
        Ok(())
    }

    pub fn next_level(&mut self, device: &wgpu::Device) -> Result<(), LevelLoadError> {
        self.level_index += 1;
        self.level_index %= MAIN_LEVELS.len();
        self.load_level(device, MAIN_LEVELS[self.level_index])
    }

    pub fn spawn_player(&mut self) {
        if let Some(spawn_point) = self.level.tilemap.get_spawn_point() {
            self.player.set_position(spawn_point);
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GameLoadError {
    #[error("failed to load level: {0}")]
    Level(#[from] LevelLoadError),
}
