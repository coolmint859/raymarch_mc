use std::sync::Arc;
use winit::window::{CursorGrabMode, Window};

use crate::graphics::GpuHandle;

/// Descriptor for the current canvas settings
#[derive(Clone, Copy)]
pub struct CanvasDescriptor {
    pub width: u32,
    pub height: u32,
    pub aspect: f32
}

impl CanvasDescriptor {
    /// Update the canvas descriptor dimensions
    pub fn update_dimensions(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.aspect = (width as f32) / (height as f32);
    }
}

/// Represents the window and rendering surface for which to issue draw commands
pub struct Canvas {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub desc: CanvasDescriptor,

    pub is_cursor_locked: bool,
}

impl Canvas {
    /// Resize the canvas to fit the window surface
    pub fn resize(&mut self, gpu: &GpuHandle, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.desc.update_dimensions(width, height);
            self.surface.configure(&gpu.device, &self.config);
        }
    }

    /// Reset the window to match the configuration width and height
    pub fn reset(&mut self, gpu: &GpuHandle) {
        self.resize(gpu, self.config.width, self.config.height);
    }

    /// Get a reference to the current canvas information
    pub fn info(&self) -> &CanvasDescriptor {
        &self.desc
    }

    /// Set the window cursor lock status
    pub fn set_cursor_lock(&mut self, lock: bool) {
        if lock {
            if self.window.set_cursor_grab(CursorGrabMode::Locked).is_ok() 
                || self.window.set_cursor_grab(CursorGrabMode::Confined).is_ok()
            {
                self.window.set_cursor_visible(false);
                self.is_cursor_locked = true;
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            self.window.set_cursor_visible(true);
            self.is_cursor_locked = false;
        }
    }
}