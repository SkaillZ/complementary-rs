mod game;
mod imgui_helpers;
mod imgui_sdl2_support;
mod input;
mod math;
mod player;
mod rendering;
mod tilemap;
mod window;

use window::Window;

fn main() -> Result<(), String> {
    #[cfg(debug_assertions)]
    env_logger::builder()
        .filter(Some("complementary_rs"), log::LevelFilter::Trace)
        .init();

    #[cfg(not(debug_assertions))]
    env_logger::init();

    let mut window = Window::new().expect("Failed to create window!");
    window.run_main_loop().unwrap();
    Ok(())
}
