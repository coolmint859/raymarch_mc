use glam::*;

use crate::{graphics::{Buffer, BufferId, Graphics, Serializable, StructuredUpdate}, utils::Transform};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PerspCameraUniform {
    pub inv_view_proj: [[f32; 4]; 4],
    pub camera_postion: [f32; 3],
    pub frame: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OrthoCameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_postion: [f32; 3],
    pub frame: f32,
}

/// A camera that embodies perspective projection
pub struct PerspectiveCamera {
    pub transform: Transform,

    fov_y: f32,
    z_near: f32,
    z_far: f32,
    view_proj: Mat4,
}

impl PerspectiveCamera {
    pub fn new() -> Self {
        Self {
            transform: Transform::default(),
            fov_y: 60.0_f32.to_radians(),
            z_near: 0.01,
            z_far: 1000.0,
            view_proj: Mat4::IDENTITY,
        }
    }

    /// Update the camera's view and projection
    pub fn update(&mut self, aspect: f32) {
        let proj_mat = Mat4::perspective_lh(self.fov_y, aspect, self.z_near, self.z_far);
        self.transform.to_updated();
        // let view_mat = self.transform.to_updated().inverse();
        let view_mat = Mat4::from_quat(self.transform.get_rotation()).inverse();

        self.view_proj = proj_mat * view_mat;
    }

    /// get this camera in it's uniform representation
    pub fn to_uniform(&self, frame: u32) -> PerspCameraUniform {
        PerspCameraUniform {
            inv_view_proj: self.view_proj.inverse().to_cols_array_2d(),
            camera_postion: self.transform.get_position().to_array(),
            frame: frame as f32,
        }
    }

    /// Get the camera's current forward axis
    pub fn forward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Z).normalize()
    }

    /// Get the camera's current rightward axis
    pub fn rightward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::X).normalize()
    }

    /// Get the camera's current upward axis
    pub fn upward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Y).normalize()
    }
}

/// A camera that embodies orthographic projection
pub struct OrthographicCamera {
    pub transform: Transform,
    buf_id: BufferId,

    z_near: f32,
    z_far: f32,
    view_proj: Mat4,
}

impl OrthographicCamera {
    pub fn new() -> Self {
        Self {
            transform: Transform::default(),
            buf_id: BufferId("ortho_camera"),
            z_near: 0.01,
            z_far: 100.0,
            view_proj: Mat4::IDENTITY,
        }
    }

    pub fn init(&mut self, graphics: &mut Graphics) {
        graphics.context.request_buffer(
            &self.buf_id, 
            Buffer::as_uniform()
                .with_struct_data(self.to_uniform(graphics.canvas.frame_count()))
                .writable()
        );
    }

    /// Update the camera's view and projection
    pub fn update(&mut self, graphics: &mut Graphics) {
        let aspect = graphics.canvas.aspect();

        let l = -aspect;
        let r = aspect;
        let b = -1.0;
        let t = 1.0;
        let proj_mat = Mat4::orthographic_lh(l, r, b, t, self.z_near, self.z_far);
        let view_mat = self.transform.to_updated();

        self.view_proj = proj_mat * view_mat;

        let _ = graphics.context.update_buffer(&self.buf_id, StructuredUpdate {
            data: &self.to_uniform(graphics.canvas.frame_count()),
            offset: 0
        });
    }

    /// get this camera in it's uniform representation
    pub fn to_uniform(&self, frame: u32) -> OrthoCameraUniform {
        OrthoCameraUniform {
            view_proj: self.view_proj.to_cols_array_2d(),
            camera_postion: self.transform.get_position().to_array(),
            frame: frame as f32,
        }
    }

    /// Get the camera's current forward axis
    pub fn forward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Z).normalize()
    }

    /// Get the camera's current rightward axis
    pub fn rightward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::X).normalize()
    }

    /// Get the camera's current upward axis
    pub fn upward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Y).normalize()
    }
}

/// Represents the view and projection of a camera 
pub trait CameraSpace {
    type Uniform: Serializable;
    /// Get the view-projection matrix associated with this camera system
    fn view_proj_mat(&self, graphics: &Graphics, dt: f32) -> Mat4;

    /// Get the POD uniform struct this camera system represents
    fn to_uniform(&self, graphics: &Graphics, dt: f32) -> Self::Uniform;

    /// Get the label that best represents this camera space.
    fn label(&self) -> &'static str;
}

/// Represents transformations for scene geometry within a space S.
pub struct Camera<S: CameraSpace> {
    /// the id to this camera's buffer
    buf_id: BufferId,
    /// the space the camera works in (view-projection)
    space: S,
}

impl<S: CameraSpace> Camera<S> {
    pub fn new(space: S) -> Self {
        Self {
            buf_id: BufferId(space.label()),
            space,
        }
    }

    /// Initialize the uniform buffer this camera uses on the gpu
    pub fn init(&mut self, graphics: &mut Graphics) {
        graphics.context.request_buffer(
            &self.buf_id, 
            Buffer::as_uniform()
                .with_struct_data(self.space.to_uniform(graphics, 0.0))
                .writable()
        );
    }

    /// Update the uniform buffer with the view-projection matrix this camera uses
    pub fn update(&mut self, graphics: &mut Graphics, dt: f32) {
        let _ = graphics.context.update_buffer(&self.buf_id, StructuredUpdate {
            data: &self.space.to_uniform(graphics, dt),
            offset: 0
        });
    }

    /// Get the unique buffer id for this camera
    pub fn buf_id(&self) -> &BufferId {
        &self.buf_id
    }
}

/// A camera space that represents the canvas drawing surface.
/// 
/// This provides a rendering space where (0,0) is in the bottom left corner, 
/// and (0, 1) is in the top left corner. 
/// 
/// For non-square canvases, the x-axis is scaled by the aspect ratio.
/// 
/// This is best used for UIs or HUDs.
pub struct ScreenSpace;

impl CameraSpace for ScreenSpace {
    type Uniform = OrthoCameraUniform;

    fn view_proj_mat(&self, graphics: &Graphics, _dt: f32) -> Mat4 {
        let aspect = graphics.canvas.aspect();

        Mat4::orthographic_lh(0.0, aspect, 0.0, 1.0, -1.0, 1.0)
    }

    fn to_uniform(&self, graphics: &Graphics, dt: f32) -> Self::Uniform {
        OrthoCameraUniform {
            view_proj: self.view_proj_mat(graphics, dt).to_cols_array_2d(),
            camera_postion: Vec3::ZERO.to_array(),
            frame: graphics.canvas.frame_count() as f32
        }
    }

    fn label(&self) -> &'static str {
        "screen_space_camera"
    }
}