use std::sync::Arc;

use winit::window::Window;

use crate::{canvas::Canvas, graphics::CanvasDescriptor};

/// A lightwight handle representing the physical gpu
#[derive(Clone, Debug)]
pub struct GpuHandle {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

/// Initialize the graphics context, creating a gpu handle and rendering canvas
pub async fn init_graphics(window: Arc<Window>) -> (GpuHandle, Canvas) {
    let size = window.inner_size();
    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window.clone()).unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .unwrap();

    let gpu = GpuHandle {
        device: Arc::new(device),
        queue: Arc::new(queue)
    };

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats[0];
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&gpu.device, &config);

    let canvas = Canvas {
        desc: CanvasDescriptor {
            width: config.width,
            height: config.height,
            aspect: (config.width as f32) / (config.height as f32)
        },
        window,
        surface,
        config,
    };

    (gpu, canvas)
}
