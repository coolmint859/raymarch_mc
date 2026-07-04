use std::sync::Arc;

use winit::window::Window;

use crate::{Canvas, graphics::GpuContext};

/// Initialize the graphics context, creating a gpu context and rendering canvas
pub async fn init_graphics(window: Arc<Window>) -> (GpuContext, Canvas) {
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
    surface.configure(&device, &config);

    let canvas = Canvas {
        window,
        surface,
        aspect: (config.width as f32) / (config.height as f32),
        config,
        is_cursor_locked: false,
        is_focused: true,
    };

    let gpu = GpuContext::new(device, queue);

    (gpu, canvas)
}
