mod game;
mod imgui_helpers;
mod imgui_sdl2_support;
mod input;
mod level;
mod math;
mod player;
mod rendering;
mod tilemap;
mod window;

use std::error::Error;

use window::Window;

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(debug_assertions)]
    env_logger::builder()
        .filter(Some("complementary_rs"), log::LevelFilter::Trace)
        .init();

    #[cfg(not(debug_assertions))]
    env_logger::init();

    let mut window = Window::new()?;
    window.run_main_loop()?;
    Ok(())
}
