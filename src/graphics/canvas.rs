use std::sync::Arc;

use winit::window::Window;

use crate::graphics::{
    RenderTarget,
    GpuHandle
};

/// A transient handle representing a single rendering frame
pub struct CanvasFrame {
    output: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    format: wgpu::TextureFormat,
}

impl RenderTarget for CanvasFrame {
    fn get_view(&self) -> &wgpu::TextureView {
        &self.view
    }

    fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    fn present(self) {
        self.output.present();
    }
}

/// Represents the window and rendering surface for which to issue draw commands
pub struct Canvas {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub aspect_ratio: f32,
}

impl Canvas {
    /// Resize the canvas to fit the window surface
    pub fn resize(&mut self, gpu: &GpuHandle, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.aspect_ratio = (width as f32) / (height as f32);
            self.surface.configure(&gpu.device, &self.config);
        }
    }

    /// Reset the window to match the configuration width and height
    pub fn reset_canvas(&mut self, gpu: &GpuHandle) {
        self.resize(gpu, self.config.width, self.config.height);
    }

    /// Get the next rendering frame from the canvas.
    pub fn get_next_frame(&self) -> Result<CanvasFrame, wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let format = self.config.format;

        Ok(CanvasFrame { output, view, format })
    }
}