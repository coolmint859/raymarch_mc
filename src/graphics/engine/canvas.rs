use std::sync::Arc;
use winit::window::{CursorGrabMode, Window};

/// Represents the window and rendering surface for which to issue draw commands
pub struct Canvas {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub aspect: f32,

    pub is_cursor_locked: bool,
    pub is_focused: bool,
}

impl Canvas {
    /// Resize the canvas to fit the window surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.aspect = (width as f32) / (height as f32);
        }
    }

    /// Reset the window to match the configuration width and height
    pub fn reset(&mut self) {
        self.resize( self.config.width, self.config.height);
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