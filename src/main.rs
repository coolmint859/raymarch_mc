use std::sync::Arc;
use std::time::Instant;

use glam::{Quat, Vec3};
use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::{DeviceEvent, DeviceId, MouseButton, WindowEvent}, event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop}, keyboard::KeyCode, window::{CursorGrabMode, WindowAttributes, WindowId},
};

pub mod game;
pub mod controls;

pub mod graphics;
use crate::{controls::{CameraController, KeyboardHandler, MouseHandler}, graphics::*};

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
    /// The renderer used to present frames to the canvas
    renderer: Option<Renderer>,
    canvas: Option<Canvas>,
    gpu: Option<GpuHandle>,

    camera: PerspectiveCamera,
    cam_default_pos: Vec3,

    controller: CameraController,
    keyboard: KeyboardHandler<KeyAction>,
    mouse: MouseHandler<MouseAction>,
    is_focused: bool,
    is_cursor_locked: bool,

    /// The time when the last frame was run
    previous_time: Instant,
    /// The total elasped time since launch
    elapsed_time: f32,
}

impl App {
    pub fn new() -> Self {
        let cam_default_pos = Vec3 {x: 0.0, y: 0.0, z: -3.0};
        let mut camera = PerspectiveCamera::new();
        camera.transform.move_to(cam_default_pos);

        Self {
            renderer: None,
            gpu: None, 
            canvas: None,
            keyboard: KeyboardHandler::new(),
            mouse: MouseHandler::new(),
            camera,
            cam_default_pos,
            is_focused: true,
            is_cursor_locked: false,
            controller: CameraController::new(5.0, 0.003),
            previous_time: Instant::now(),
            elapsed_time: 0.0,
        }
    }

    /// Initialize the app
    pub fn init(&mut self) {
        self.keyboard.register_key(KeyCode::KeyW, KeyAction::MoveForward);
        self.keyboard.register_key(KeyCode::KeyA, KeyAction::StrafeLeft);
        self.keyboard.register_key(KeyCode::KeyS, KeyAction::MoveBackward);
        self.keyboard.register_key(KeyCode::KeyD, KeyAction::StrafeRight);
        self.keyboard.register_key(KeyCode::ShiftLeft, KeyAction::MoveUp);
        self.keyboard.register_key(KeyCode::Space, KeyAction::MoveDown);
        self.keyboard.register_key(KeyCode::Escape, KeyAction::Exit);
        self.keyboard.register_key(KeyCode::KeyR, KeyAction::ResetCamera);

        self.mouse.register_button(MouseButton::Left, MouseAction::LockMouse);
        self.mouse.register_button(MouseButton::Right, MouseAction::UnlockMouse);
    }

    pub fn process_input(&mut self, event_loop: &ActiveEventLoop, dt: f32) {
        if self.is_focused && self.is_cursor_locked {
            let dm = self.mouse.poll_motion();
            if dm.dx != 0.0 || dm.dy != 0.0 {
                self.controller.rotate_delta(&mut self.camera, dm.dx, dm.dy);
            }

            for action in self.keyboard.poll_on_held() {
                match action {
                    KeyAction::MoveForward => self.controller.move_forward(&mut self.camera, dt),
                    KeyAction::MoveBackward => self.controller.move_backward(&mut self.camera, dt),
                    KeyAction::StrafeLeft => self.controller.strafe_left(&mut self.camera, dt),
                    KeyAction::StrafeRight => self.controller.strafe_right(&mut self.camera, dt),
                    KeyAction::MoveUp => self.controller.move_up(&mut self.camera, dt),
                    KeyAction::MoveDown => self.controller.move_down(&mut self.camera, dt),
                    KeyAction::ResetCamera => {
                        self.camera.transform.move_to(self.cam_default_pos);
                        self.camera.transform.set_rotation(Quat::IDENTITY);
                        self.controller.reset_delta();
                    },
                    _ => {}
                }
            }
        }

        for action in self.keyboard.poll_on_press() {
            match action {
                KeyAction::Exit => event_loop.exit(),
                _ => {}
            }
        }

        for action in self.mouse.poll_on_press() {
            match action {
                MouseAction::LockMouse => self.set_cursor_lock(true),
                MouseAction::UnlockMouse => self.set_cursor_lock(false),
            }
        }

        self.keyboard.clear_events();
        self.mouse.clear_events();
    }

    pub fn set_cursor_lock(&mut self, lock: bool) {
        let Some(canvas) = &mut self.canvas else { return };

        if lock {
            if canvas.window.set_cursor_grab(CursorGrabMode::Locked).is_ok() 
                || canvas.window.set_cursor_grab(CursorGrabMode::Confined).is_ok()
            {
                canvas.window.set_cursor_visible(false);
                self.is_cursor_locked = true;
            }
        } else {
            let _ = canvas.window.set_cursor_grab(CursorGrabMode::None);
            canvas.window.set_cursor_visible(true);
            self.is_cursor_locked = false;
        }
    }
    
    pub fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        self.process_input(event_loop, dt);

        let (Some(canvas), Some(renderer), Some(gpu)) 
            = (&mut self.canvas, &mut self.renderer, &self.gpu) 
            else { return; };

        renderer.update_camera(gpu.clone(), &mut self.camera, canvas.info().aspect);

        canvas.window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_inner_size(Size::Physical (
                    PhysicalSize { width: 1920, height: 1080 }
                ))
                .with_title("Ray Marching!");
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            event_loop.listen_device_events(DeviceEvents::Always);

            let (gpu, canvas) = pollster::block_on(init_graphics(window));
            
            self.renderer = Some(Renderer::new(&gpu, &canvas.config));
            self.gpu = Some(gpu);
            self.canvas = Some(canvas);

            self.init();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.run_frame(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let (Some(canvas), Some(renderer), Some(gpu)) 
            = (&mut self.canvas, &mut self.renderer, &self.gpu) 
            else { return; };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                canvas.resize(&gpu, physical_size.width, physical_size.height);
            }
            WindowEvent::RedrawRequested => {
                match canvas.get_next_frame() {
                    Ok(frame) => {
                        let commands = renderer.render(gpu.clone(), &frame);

                        gpu.queue.submit(std::iter::once(commands));
                        frame.present();
                    }
                    Err(wgpu::SurfaceError::Lost) => canvas.reset_canvas(&gpu),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("{e:?}")
                }
                canvas.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.keyboard.key_event(&event);
            }
            WindowEvent::Focused(focused) => {
                self.is_focused = focused;
                self.set_cursor_lock(focused);
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
