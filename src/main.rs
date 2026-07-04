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

/// Core window driver
struct App {
    graphics: Option<Graphics>,
    active_screen: Option<Box<dyn Screen>>,

    previous_time: Instant,
    elapsed_time: f32,
}

impl App {
    pub fn new() -> Self {
        Self {
            graphics: None,
            active_screen: None,
            previous_time: Instant::now(),
            elapsed_time: 0.0,
        }
    }
    
    pub fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
        let Some(graphics) = &mut self.graphics else { return; };

        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        if graphics.canvas.is_focused && let Some(ref mut screen) = self.active_screen {
            match screen.process_input(graphics, dt) {
                ScreenTransition::Exit => {
                    event_loop.exit();
                    return;
                }
                ScreenTransition::SwitchTo(mut screen) => {
                    screen.init(graphics);
                    self.active_screen = Some(screen);
                    return;
                }
                ScreenTransition::None => {
                    screen.update(graphics, dt)
                }
            }
        }

        graphics.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_inner_size(Size::Physical (
                    PhysicalSize { width: 2560, height: 1440 }
                ))
                .with_title("Ray Marching!");
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            event_loop.listen_device_events(DeviceEvents::Always);

            let mut graphics_init = GraphicsInit::new();
            let mut graphics = pollster::block_on(graphics_init.init(window)).unwrap();

            let mut game_screen = Game::new();
            game_screen.init(&mut graphics);

            self.active_screen = Some(Box::new(game_screen));
            self.graphics = Some(graphics);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.run_frame(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let (Some(graphics), Some(screen)) = (&mut self.graphics, &mut self.active_screen) else { return; };

        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {        
                graphics.on_resize(physical_size.width, physical_size.height);
                screen.on_resize(graphics);

                screen.update(graphics, dt);
            }
            WindowEvent::RedrawRequested => {
                let result = screen.render(graphics);

                // println!("{:#?}", passes);
                match result {
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Lost) => graphics.reset(),
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                screen.input_event(InputEvent::Key(event));
            }
            WindowEvent::Focused(focused) => {
                graphics.canvas.is_focused = focused;
                graphics.canvas.set_cursor_lock(focused);
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
