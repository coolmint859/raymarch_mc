use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::dpi::Size;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;
use winit::window::WindowAttributes;
use winit::window::WindowId;

pub mod graphics;
use crate::graphics::*;

pub mod game;

/// Core window driver
struct App {
    /// The renderer used to present frames to the canvas
    renderer: Option<Renderer>,
    canvas: Option<Canvas>,
    gpu: Option<GpuHandle>,

    /// The time when the last frame was run
    previous_time: Instant,
    /// The total elasped time since launch
    elapsed_time: f32,
}

impl App {
    pub fn new() -> Self {
        Self {
            renderer: None,
            gpu: None, 
            canvas: None,
            previous_time: Instant::now(),
            elapsed_time: 0.0,
        }
    }
    
    pub fn run_frame(&mut self) {
        let (Some(canvas), Some(renderer), Some(gpu)) 
            = (&mut self.canvas, &mut self.renderer, &self.gpu) 
            else { return; };

        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;
        self.elapsed_time += dt;

        renderer.update_camera(gpu.clone(), canvas, self.elapsed_time);

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

            let (gpu, canvas) = pollster::block_on(init_graphics(window));
            
            self.renderer = Some(Renderer::new(&gpu, &canvas.config));
            self.gpu = Some(gpu);
            self.canvas = Some(canvas);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.run_frame();
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
                let key_state = event.state;
                let code = event.physical_key;

                match (code, key_state.is_pressed()) {
                    (PhysicalKey::Code(KeyCode::Escape), true) => event_loop.exit(),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
