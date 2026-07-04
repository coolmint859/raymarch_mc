use std::sync::Arc;
use std::time::Instant;

use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent}, event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop}, window::{WindowAttributes, WindowId},
};

pub mod game;
pub mod controls;

pub mod graphics;
use crate::{game::{Game, Screen, ScreenTransition}, graphics::*};

/// Represents events triggered by user input
pub enum InputEvent {
    /// User pressed a key
    Key(KeyEvent),
    /// User pressed a mouse button
    MouseButton{
        state: ElementState,
        button: MouseButton
    },
    /// User moved the mouse
    MouseMotion{
        dx: f64, 
        dy: f64
    }
}

/// Contains the gpu context and rendering canvas
pub struct AppEnv {
    pub gpu: GpuContext,
    pub canvas: Canvas,
}

/// Core window driver
struct App {
    env: Option<AppEnv>,
    active_screen: Option<Box<dyn Screen>>,

    previous_time: Instant,
    elapsed_time: f32,
}

impl App {
    pub fn new() -> Self {
        Self {
            env: None,
            active_screen: None,
            previous_time: Instant::now(),
            elapsed_time: 0.0,
        }
    }
    
    pub fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
        let Some(env) = &mut self.env else { return; };

        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        if env.canvas.is_focused && let Some(ref mut screen) = self.active_screen {
            match screen.process_input(env, dt) {
                ScreenTransition::Exit => {
                    event_loop.exit();
                    return;
                }
                ScreenTransition::SwitchTo(mut screen) => {
                    screen.init(env);
                    self.active_screen = Some(screen);
                    return;
                }
                ScreenTransition::None => {
                    screen.update(env, dt)
                }
            }
        }

        env.canvas.window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.env.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_inner_size(Size::Physical (
                    PhysicalSize { width: 2560, height: 1440 }
                ))
                .with_title("Ray Marching!");
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            event_loop.listen_device_events(DeviceEvents::Always);

            let (gpu, canvas) = pollster::block_on(init_graphics(window));
            let mut app_env = AppEnv { gpu, canvas };

            let mut game_screen = Game::new();
            game_screen.init(&mut app_env);

            self.active_screen = Some(Box::new(game_screen));
            self.env = Some(app_env);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.run_frame(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let (Some(env), Some(screen)) = (&mut self.env, &mut self.active_screen) else { return; };

        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {        
                env.canvas.resize(physical_size.width, physical_size.height);
                env.gpu.configure_surface(&mut env.canvas);

                screen.on_resize(env);
                screen.update(env, dt);
            }
            WindowEvent::RedrawRequested => {
                let passes = screen.render(env);

                // println!("{:#?}", passes);
                match env.gpu.execute_passes(passes, &mut env.canvas) {
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Lost) => env.canvas.reset(),
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                screen.input_event(InputEvent::Key(event));
            }
            WindowEvent::Focused(focused) => {
                env.canvas.is_focused = focused;
                env.canvas.set_cursor_lock(focused);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                screen.input_event(InputEvent::MouseButton { state, button });
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        let Some(ref mut screen) = self.active_screen else { return; };
        
        match event {
            DeviceEvent::MouseMotion { delta } => {
                screen.input_event(InputEvent::MouseMotion { dx: delta.0, dy: delta.1 });
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    env_logger::init();

    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
