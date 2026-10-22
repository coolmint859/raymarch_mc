use std::sync::Arc;
use winit::window::{CursorGrabMode, Window};

/// The width and height of the `Canvas`
pub struct CanvasDimensions {
    pub width: u32,
    pub height: u32
}

/// A snapshot (frame) of the `Canvas`.
pub struct CanvasFrame {
    /// The rendering surface
    surface: wgpu::SurfaceTexture,
    /// The current wgpu texture view onto the `Canvas`
    pub view: wgpu::TextureView,
    /// The current frame number
    pub number: u32,
    /// The canvas aspect ratio
    pub aspect: f32,
    /// the width/height of the `Canvas`
    pub dimensions: CanvasDimensions
}

impl CanvasFrame {
    /// Present the frame to the surface. This consumes the frame.
    pub fn present(self) {
        self.surface.present();
    }
}

/// Represents the window and rendering surface
pub struct Canvas {
    pub(crate) window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    aspect: f32,

    is_cursor_locked: bool,
    is_focused: bool,
    frame_number: u32,
}

impl Canvas {
    pub fn new(
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let aspect = config.width as f32 / config.height as f32;
        Self {
            window,
            surface, 
            config,
            aspect,
            is_cursor_locked: false,
            is_focused: false,
            frame_number: 0,
        }
    }

    /// Resize the canvas to fit the window surface
    pub fn resize(&mut self, width: u32, height: u32, device: &wgpu::Device) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.aspect = (width as f32) / (height as f32);

            self.surface.configure(device, &self.config);
        }
    }

    /// Reset the canvas window to match the configuration width and height
    pub fn reset(&mut self, device: &wgpu::Device) {
        self.resize( self.config.width, self.config.height, device);
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

    /// Retrieve the next rendering frame. This increments the frame counter.
    pub fn next_frame(&mut self) -> Result<CanvasFrame, wgpu::SurfaceError> {
        let surface = self.surface.get_current_texture()?;
        let view = surface.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let curr_frame = self.frame_number;
        
        self.frame_number += 1;

        Ok(CanvasFrame { 
            surface, 
            view, 
            number: curr_frame, 
            aspect: self.aspect,
            dimensions: self.dimensions()
        })
    }

    /// Returns the current dimensions of the canvas in pixels
    pub fn dimensions(&self) -> CanvasDimensions {
        CanvasDimensions { width: self.config.width, height: self.config.height }
    }

    /// Returns the current width of the canvas in pixels
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Returns the current height of the canvas in pixels
    pub fn height(&self) -> u32 {
        self.config.height
    }

    /// Get the aspect ratio of the canvas
    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    /// Get the number of frames presented since application start
    pub fn frame_count(&self) -> u32 {
        self.frame_number
    }

    /// Check if the canvas has the pointer focused on it.
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Check if the cursor is locked to the canvas
    pub fn cursor_locked(&self) -> bool {
        self.is_cursor_locked
    }
    
    /// request the window to trigger a redraw event
    pub fn redraw(&self) {
        self.window.request_redraw();
    }

    /// Set the canvas to be in the focused state
    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }
}