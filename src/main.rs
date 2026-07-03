use std::sync::Arc;
use std::time::Instant;

use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::{DeviceEvent, DeviceId, WindowEvent}, event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop}, window::{WindowAttributes, WindowId},
};

pub mod game;
pub mod controls;

pub mod graphics;
use crate::{controls::{KeyboardHandler, MouseHandler}, game::Game, graphics::*};

#[derive(Clone, Copy)]
pub enum KeyAction {
    MoveForward,
    MoveBackward,
    StrafeLeft, 
    StrafeRight,
    MoveUp,
    MoveDown,
    ResetCamera,
    Exit,
}

#[derive(Clone, Copy)]
pub enum MouseAction {
    LockMouse,
    UnlockMouse,
}

/// Core window driver
struct App {
    gpu_ctx: Option<GpuContext>,
    canvas: Option<Canvas>,
    game: Option<Game>,

    keyboard: KeyboardHandler<KeyAction>,
    mouse: MouseHandler<MouseAction>,
    is_focused: bool,

    previous_time: Instant,
    elapsed_time: f32,
}

impl App {
    pub fn new() -> Self {
        Self {
            gpu_ctx: None,
            canvas: None,
            game: None,
            keyboard: KeyboardHandler::new(),
            mouse: MouseHandler::new(),
            is_focused: true,
            previous_time: Instant::now(),
            elapsed_time: 0.0,
        }
    }
    
    pub fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(canvas), Some(context), Some(game)) 
            = (&mut self.canvas, &mut self.gpu_ctx, &mut self.game) 
            else { return; };

        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        for action in self.keyboard.peek_on_press() {
            match action {
                KeyAction::Exit => event_loop.exit(),
                _ => {}
            }
        }

        for action in self.mouse.peek_on_press() {
            match action {
                MouseAction::LockMouse => canvas.set_cursor_lock(true),
                MouseAction::UnlockMouse => canvas.set_cursor_lock(false),
            }
        }

        if canvas.is_cursor_locked && self.is_focused {
            game.process_input(&mut self.keyboard, &mut self.mouse, dt);
        }

        game.update(context, canvas, dt);

        self.keyboard.clear_events();
        self.mouse.clear_events();

        canvas.window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_ctx.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_inner_size(Size::Physical (
                    PhysicalSize { width: 2560, height: 1440 }
                ))
                .with_title("Ray Marching!");
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            event_loop.listen_device_events(DeviceEvents::Always);

            let (gpu, canvas) = pollster::block_on(init_graphics(window));
            let mut gpu_ctx = GpuContext::new(&gpu);

            let game = Game::init(&mut gpu_ctx, &canvas.config);
            game.init_input(&mut self.keyboard, &mut self.mouse);

            self.game = Some(game);
            self.gpu_ctx = Some(gpu_ctx);
            self.canvas = Some(canvas);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.run_frame(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let (Some(canvas), Some(gpu_ctx), Some(game)) 
            = (&mut self.canvas, &mut self.gpu_ctx, &mut self.game) 
            else { return; };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                canvas.resize(&gpu_ctx.gpu, physical_size.width, physical_size.height);
                game.on_resize(gpu_ctx, canvas);
                self.run_frame(event_loop);
            }
            WindowEvent::RedrawRequested => {
                let passes = game.create_passes(canvas);
                match gpu_ctx.execute_passes(passes, canvas) {
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Lost) => canvas.reset(&gpu_ctx.gpu),
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.keyboard.key_event(&event);
            }
            WindowEvent::Focused(focused) => {
                self.is_focused = focused;
                canvas.set_cursor_lock(focused);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.mouse.button_event(state, button);
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.mouse.motion_event(delta.0, delta.1);
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
