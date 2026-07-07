#![allow(dead_code)]
use std::sync::Arc;

use winit::window::Window;

use crate::{Canvas, graphics::GpuContext};

/// Graphics initialization errors
#[derive(Clone, Debug)]
pub enum GraphicsInitError {
    /// Surface Failed to be created
    Surface(wgpu::CreateSurfaceError),
    /// Adapter failed requesteding
    Adapter(wgpu::RequestAdapterError),
    /// Logical Device failed requesting
    Device(wgpu::RequestDeviceError),
    /// Graphics have already been initialized
    Initialized,
}

impl From<wgpu::CreateSurfaceError> for GraphicsInitError {
    fn from(err: wgpu::CreateSurfaceError) -> Self {
        GraphicsInitError::Surface(err)
    }
}

impl From<wgpu::RequestAdapterError> for GraphicsInitError {
    fn from(err: wgpu::RequestAdapterError) -> Self {
        GraphicsInitError::Adapter(err)
    }
}

impl From<wgpu::RequestDeviceError> for GraphicsInitError {
    fn from(err: wgpu::RequestDeviceError) -> Self {
        GraphicsInitError::Device(err)
    }
}

/// Contains the gpu context and rendering canvas
pub struct Graphics {
    pub gpu: GpuContext,
    pub canvas: Canvas,
}

impl Graphics {
    /// Update the graphics state when the user changes the window size
    pub fn on_resize(&mut self, width: u32, height: u32) {
        self.canvas.resize(width, height);
        self.gpu.configure_surface(&mut self.canvas);
    }

    /// Reset the graphics state
    pub fn reset(&mut self) {
        self.canvas.reset();
    }

    /// Request the canvas window for a redraw
    pub fn request_redraw(&self) {
        self.canvas.window.request_redraw();
    }
}

/// Handle to the gpu device and queue
#[derive(Clone, Debug)]
pub struct GpuHandle {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue
}

/// Initializes the graphics environment for a given window
pub struct GraphicsInit {
    power_pref: wgpu::PowerPreference,
    present_mode: wgpu::PresentMode,
    frame_latency: u32,
    back_end: wgpu::Backends,
    is_initialized: bool
}

impl GraphicsInit {
    pub fn new() -> Self {
        Self {
            power_pref: wgpu::PowerPreference::None,
            present_mode: wgpu::PresentMode::Fifo,
            frame_latency: 2,
            back_end: wgpu::Backends::PRIMARY,
            is_initialized: false
        }
    }

    /// Set the power preference for the instance adapter
    pub fn with_power_pref(mut self, power_pref: wgpu::PowerPreference) -> Self {
        self.power_pref = power_pref;
        self
    }

    /// Set the presentation mode for the rendering surface
    pub fn with_present_mode(mut self, present_mode: wgpu::PresentMode) -> Self {
        self.present_mode = present_mode;
        self
    }

    /// Set the max frame latency between surface texture requests and surface presentation.
    pub fn with_max_latency(mut self, latency: u32) -> Self {
        self.frame_latency = latency;
        self
    }

    /// Set the target graphics api backend. This is platform specific.
    pub fn with_backend(mut self, backend: wgpu::Backends) -> Self {
        self.back_end = backend;
        self
    }

    /// Inititialize the graphics environment with the provided window.
    /// 
    /// This method should only be called once. Subsequent calls result in a error
    pub async fn init(&mut self, window: Arc<Window>) -> Result<Graphics, GraphicsInitError> {
        if self.is_initialized { return Err(GraphicsInitError::Initialized) }

        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: self.back_end,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.power_pref,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;
        let gpu = GpuHandle { device, queue };

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: self.present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: self.frame_latency,
        };
        surface.configure(&gpu.device, &config);

        let canvas = Canvas {
            window,
            surface,
            aspect: (config.width as f32) / (config.height as f32),
            config,
            is_cursor_locked: false,
            is_focused: true,
        };

        let gpu = GpuContext::new(gpu);

        Ok(Graphics { gpu, canvas })
    }
}