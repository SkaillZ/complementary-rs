use std::time::{Duration, Instant};

use crate::game::Game;
use crate::imgui_sdl2_support::{filter_event, SdlPlatform as ImguiSdlPlatform};
use crate::input::{ButtonType, Input};
use crate::math::{FVec2, FVec3};
use cgmath::num_traits::ToPrimitive;
use imgui::FontSource;
use imgui_wgpu::{Renderer as ImguiRenderer, RendererConfig};
use log::{info, warn};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::video::Window as SdlWindow;
use sdl2::Sdl;

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use wgpu::{include_wgsl, vertex_attr_array, BufferUsages};

pub struct WindowWrapper<'a>(pub &'a SdlWindow);

unsafe impl<'a> HasRawWindowHandle for WindowWrapper<'a> {
    #[cfg(not(target_os = "macos"))]
    /// all non-mac platforms work correctly, so return the handle directly
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0.raw_window_handle()
    }

    #[cfg(target_os = "macos")]
    /// do some work on appkit to get the root NSView for the NSWindow returned by sdl2
    fn raw_window_handle(&self) -> RawWindowHandle {
        use objc::runtime::Object;
        use objc::{msg_send, sel, sel_impl};
        let handle = self.0.raw_window_handle();
        match handle {
            RawWindowHandle::AppKit(appkit_handle) => unsafe {
                let mut new_handle = appkit_handle.clone();
                new_handle.ns_view = msg_send![appkit_handle.ns_window as *mut Object, contentView];
                RawWindowHandle::AppKit(new_handle)
            },
            _ => unreachable!(),
        }
    }
}

pub struct Window {
    game: Game,
    sdl_context: Sdl,
    sdl_window: SdlWindow,

    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,

    imgui: imgui::Context,
    imgui_renderer: ImguiRenderer,
    imgui_platform: ImguiSdlPlatform,
}

pub struct DrawContext<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub output: &'a wgpu::TextureView,
    pub queue: &'a wgpu::Queue,
    pub window_width: u32,
    pub window_height: u32,
}

impl Window {
    pub fn new() -> Result<Window, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let sdl_window = video_subsystem
            .window("Complementary", 800, 600)
            .position_centered()
            .resizable()
            .allow_highdpi()
            .build()
            .map_err(|e| e.to_string())?;

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let wrapper = WindowWrapper(&sdl_window);
        let surface = unsafe { instance.create_surface(&wrapper) };

        let adapter_opt =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }));
        let adapter = match adapter_opt {
            Some(a) => a,
            None => return Err(String::from("No adapter found")),
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                limits: wgpu::Limits::default(),
                label: Some("device"),
                features: wgpu::Features::empty(),
            },
            None,
        )) {
            Ok(a) => a,
            Err(e) => return Err(e.to_string()),
        };

        let game = Game::new(&device).map_err(|e| e.to_string())?;

        let (width, height) = sdl_window.drawable_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width,
            height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&device, &surface_config);

        // Set up dear imgui
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let font_size = 13.0 as f32;
        imgui.io_mut().font_global_scale = 1.0 as f32;

        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        let renderer_config = RendererConfig {
            texture_format: surface_config.format,
            ..Default::default()
        };

        let imgui_platform = ImguiSdlPlatform::init(&mut imgui);
        let imgui_renderer = ImguiRenderer::new(&mut imgui, &device, &queue, renderer_config);

        Ok(Window {
            game,
            sdl_window,
            sdl_context,

            device,
            queue,
            surface,
            surface_config,

            imgui,
            imgui_platform,
            imgui_renderer,
        })
    }

    pub fn run_main_loop(&mut self) -> Result<(), String> {
        let mut input = Input::new();

        let mut last_frame_time = Instant::now();
        let mut lag = Duration::default();

        let mut event_pump = self.sdl_context.event_pump()?;
        'running: loop {
            for event in event_pump.poll_iter() {
                self.imgui_platform.handle_event(&mut self.imgui, &event);

                match event {
                    Event::Window {
                        window_id,
                        win_event: WindowEvent::SizeChanged(..),
                        ..
                    } if window_id == self.sdl_window.id() => {
                        let (width, height) = self.sdl_window.drawable_size();
                        self.surface_config.width = width;
                        self.surface_config.height = height;
                        self.surface.configure(&self.device, &self.surface_config);
                    }
                    Event::Quit { .. } => {
                        break 'running;
                    }
                    Event::KeyDown {
                        keycode: Some(keycode),
                        repeat: false,
                        ..
                    } => match keycode {
                        Keycode::Space => {
                            input.set_button_pressed(ButtonType::Jump);
                            input.set_button_pressed(ButtonType::Confirm);
                        }
                        Keycode::Return => {
                            input.set_button_pressed(ButtonType::Switch);
                            input.set_button_pressed(ButtonType::Confirm);
                        }
                        Keycode::RShift => input.set_button_pressed(ButtonType::SwitchAndAbility),
                        Keycode::RCtrl | Keycode::RAlt => {
                            input.set_button_pressed(ButtonType::Ability)
                        }
                        Keycode::Left | Keycode::A => input.set_button_pressed(ButtonType::Left),
                        Keycode::Right | Keycode::D => input.set_button_pressed(ButtonType::Right),
                        Keycode::Up | Keycode::W => {
                            input.set_button_pressed(ButtonType::Up);
                            input.set_button_pressed(ButtonType::Jump);
                        }
                        Keycode::Down | Keycode::S => input.set_button_pressed(ButtonType::Down),
                        Keycode::Escape | Keycode::P => input.set_button_pressed(ButtonType::Pause),
                        _ => (),
                    },
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => match keycode {
                        Keycode::Space => {
                            input.set_button_released(ButtonType::Jump);
                            input.set_button_released(ButtonType::Confirm);
                        }
                        Keycode::Return => {
                            input.set_button_released(ButtonType::Switch);
                            input.set_button_released(ButtonType::Confirm);
                        }
                        Keycode::RShift => input.set_button_released(ButtonType::SwitchAndAbility),
                        Keycode::RCtrl | Keycode::RAlt => {
                            input.set_button_released(ButtonType::Ability)
                        }
                        Keycode::Left | Keycode::A => input.set_button_released(ButtonType::Left),
                        Keycode::Right | Keycode::D => input.set_button_released(ButtonType::Right),
                        Keycode::Up | Keycode::W => {
                            input.set_button_released(ButtonType::Up);
                            input.set_button_released(ButtonType::Jump);
                        }
                        Keycode::Down | Keycode::S => input.set_button_released(ButtonType::Down),
                        Keycode::Escape | Keycode::P => {
                            input.set_button_released(ButtonType::Pause)
                        }
                        _ => (),
                    },

                    _e => {
                        //dbg!(e);
                    }
                }
            }

            let elapsed = last_frame_time.elapsed();
            lag += elapsed;
            last_frame_time = Instant::now();

            let mut frame_tick_count = 0;
            while lag >= Game::TICK_DURATION {
                lag -= Game::TICK_DURATION;

                input.tick();
                self.game.tick(&input, &self.device);

                frame_tick_count += 1;

                // Only loop ticks up until MAX_TICKS_PER_FRAME to avoid getting stuck forever
                if frame_tick_count > Game::MAX_TICKS_PER_FRAME {
                    let skipped_frame_count = lag.as_nanos() / Game::TICK_DURATION.as_nanos();
                    lag -= Game::TICK_DURATION * (skipped_frame_count.to_u32().unwrap_or(u32::MAX));
                    warn!("Lagging, skipped {skipped_frame_count} ticks");
                }
            }

            self.imgui_platform
                .prepare_frame(&mut self.imgui, &self.sdl_window, &event_pump);
            let gui_frame = self.imgui.frame();
            self.game.draw_gui(&gui_frame, &mut input, &self.device);

            let frame_res = self.surface.get_current_texture();
            let frame = match frame_res {
                Ok(a) => a,
                Err(e) => return Err(format!("Timeout getting next texture: {}", e)),
            };
            let output = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("command_encoder"),
                });

            let mut draw_context = DrawContext {
                encoder: &mut encoder,
                output: &output,
                queue: &self.queue,
                window_width: self.surface_config.width,
                window_height: self.surface_config.height,
            };

            self.game.draw(&mut draw_context);

            {
                // Imgui pass
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: &output,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                    label: None,
                });
                self.imgui_renderer
                    .render(gui_frame.render(), &self.queue, &self.device, &mut rpass)
                    .expect("Rendering failed");
            }

            self.queue.submit([encoder.finish()]);
            frame.present();
        }

        Ok(())
    }
}
