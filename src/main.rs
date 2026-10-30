use std::sync::Arc;
use std::time::Instant;

use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent}, event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop}, keyboard::KeyCode, window::{WindowAttributes, WindowId},
};

pub mod game;
pub mod utils;

pub mod graphics;
use crate::{game::{ActiveScreen, Game, QuadTest, ScreenRouter, TextTest}, graphics::*, utils::InputHandler};

/// Represents types that control the state of the window
pub trait WindowDriver {
    /// Initialize the driver state
    fn init(&mut self, graphics: &mut Graphics);

    /// Process an input event as detected by the OS
    fn process_event(&mut self, event: InputEvent);

    /// Perform resize operations as per the window
    fn on_resize(&mut self, _graphics: &mut Graphics) {}

    /// Process user input. 
    /// 
    /// Returns a `WindowState`:
    /// * `WindowState::Continue` means the app should continue processing window events.
    /// * `WindowState::Exit` means the app should close the window
    fn process_input(&mut self, _graphics: &mut Graphics, _dt: f32) -> WindowState;

    /// Update the driver state
    fn update(&mut self, graphics: &mut Graphics, dt: f32);

    /// Render the driver state to the window
    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError>;
}

/// Mouse actions global to the app
#[derive(Debug, Clone)]
enum GlobalMouseAction {
    LockMouse,
    UnlockMouse,
}

/// Keyboard actions global to the app
#[derive(Debug, Clone)]
enum GlobalKeyboardAction {
    Exit
}

/// Represents the the frame-by-frame state of the window and event loop.
pub enum WindowState {
    /// Window should remain open and continue to process events.
    Continue,
    /// Window should close.
    Exit,
}

/// Represents events triggered by user input
#[derive(Debug, Clone)]
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
    global_input: InputHandler<GlobalKeyboardAction, GlobalMouseAction>,
    driver: Option<Box<dyn WindowDriver>>,

    previous_time: Instant,
    elapsed_time: f32,
}

impl App {
    pub fn new() -> Self {
        Self {
            graphics: None,
            global_input: InputHandler::new(),

            driver: None,
            previous_time: Instant::now(),
            elapsed_time: 0.0,
        }
    }

    pub fn register_bindings(&mut self) {
        self.global_input.keyboard.register_key(KeyCode::Escape, GlobalKeyboardAction::Exit);

        self.global_input.mouse.register_button(MouseButton::Left, GlobalMouseAction::LockMouse);
        self.global_input.mouse.register_button(MouseButton::Right, GlobalMouseAction::UnlockMouse);
    }
    
    pub fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
        let Some(graphics) = &mut self.graphics else { return; };

        graphics.context.sync();

        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        if graphics.canvas.is_focused() && let Some(ref mut driver) = self.driver {
            for action in self.global_input.mouse.poll_on_press() {
                match action {
                    GlobalMouseAction::LockMouse => graphics.canvas.set_cursor_lock(true),
                    GlobalMouseAction::UnlockMouse => graphics.canvas.set_cursor_lock(false),
                }
            }

            for action in self.global_input.keyboard.poll_on_press() {
                match action {
                    GlobalKeyboardAction::Exit => event_loop.exit(),
                }
            }

            match driver.process_input(graphics, dt) {
                WindowState::Exit => {
                    event_loop.exit();
                    return;
                },
                WindowState::Continue => {
                    driver.update(graphics, dt);
                }
            }
        }

        self.global_input.clear_events();
        graphics.canvas.redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_inner_size(Size::Physical (
                    PhysicalSize { width: 1920, height: 1080 }
                ))
                .with_title("Ray Marching!");
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            event_loop.listen_device_events(DeviceEvents::Always);

            self.register_bindings();

            let mut graphics_init = GraphicsInit::new().with_backend(wgpu::Backends::DX12);
            let mut graphics = pollster::block_on(graphics_init.init(window)).unwrap();

            let mut init_screen = Game::new();
            let mut input = InputHandler::new();

            init_screen.init(&mut graphics, &mut input);

            let active = ActiveScreen::Game { 
                screen: init_screen, 
                input
            };

            self.driver = Some(Box::new(ScreenRouter::new(active)));
            self.graphics = Some(graphics);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.run_frame(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let (Some(graphics), Some(driver)) = (&mut self.graphics, &mut self.driver) else { return; };

        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {   
                // println!("---WINDOW RESIZED---");     
                graphics.on_resize(physical_size.width, physical_size.height);
                driver.on_resize(graphics);

                driver.update(graphics, dt);
                let result = driver.render(graphics);

                match result {
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Lost) => graphics.reset(),
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                match driver.render(graphics) {
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Lost) => graphics.reset(),
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let input_event = InputEvent::Key(event);

                self.global_input.process_event(input_event.clone());
                driver.process_event(input_event);
            }
            WindowEvent::Focused(focused) => {
                graphics.canvas.set_focused(focused);
                graphics.canvas.set_cursor_lock(focused);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let input_event = InputEvent::MouseButton { state, button };
                
                self.global_input.process_event(input_event.clone());
                driver.process_event(input_event);
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        let Some(ref mut screen) = self.driver else { return; };
        
        match event {
            DeviceEvent::MouseMotion { delta } => {
                let input_event = InputEvent::MouseMotion { dx: delta.0, dy: delta.1 };
                
                self.global_input.process_event(input_event.clone());
                screen.process_event(input_event);
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    env_logger::init();

    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
