use glam::*;

use crate::{graphics::{Buffer, BufferId, Graphics, StructuredUpdate}, utils::{Controllable, Transform}};

/// Raw camera uniform data
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj_mat: [[f32; 4]; 4],
    pub cam_position: [f32; 3],
    pub frame: f32,
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_proj_mat: Mat4::IDENTITY.to_cols_array_2d(),
            cam_position: [0.0; 3],
            frame: 0.0
        }
    }
}

/// Represents the view and projection of a camera 
pub trait CameraSpace {
    /// Get the label that best represents this camera space.
    fn label(&self) -> &'static str;

    /// Get the view-projection matrix associated with this camera space
    fn view_proj_mat(&self, graphics: &Graphics) -> Mat4;

    /// Get the position in world space that the camera resides at
    fn position(&self) -> Vec3;
}

/// allows cameras with controllable spaces to act like a Controllable
impl<S: CameraSpace + Controllable> Controllable for Camera<S> {
    fn transform(&self) -> &Transform {
        self.space.transform()
    }

    fn transform_mut(&mut self) -> &mut Transform {
        self.space.transform_mut()
    }

    /// Get the camera's current forward axis
    fn forward_axis(&self) -> Vec3 {
        self.space.forward_axis()
    }

    /// Get the camera's current rightward axis
    fn rightward_axis(&self) -> Vec3 {
        self.space.rightward_axis()
    }

    /// Get the camera's current upward axis
    fn upward_axis(&self) -> Vec3 {
        self.space.upward_axis()
    }
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
            buf_id: BufferId::UNINIT,
            space,
        }
    }

    /// Initialize the uniform buffer this camera uses on the gpu
    pub fn init(&mut self, graphics: &mut Graphics) {
        self.buf_id = graphics.context.request_buffer(
            Buffer::as_uniform()
                .with_label("Camera Uniform Buffer")
                .with_struct_data(self.to_uniform(graphics, 0.0))
                .writable()
        );
    }

    /// Update the uniform buffer with the view-projection matrix this camera uses
    pub fn update(&mut self, graphics: &mut Graphics, et: f32) {
        let _ = graphics.context.update_buffer(&self.buf_id, StructuredUpdate {
            data: &self.to_uniform(graphics, et),
            offset: 0
        });
    }

    /// Get the unique buffer id for this camera
    pub fn buf_id(&self) -> &BufferId {
        &self.buf_id
    }

    /// Get a reference to the space this camera works in
    pub fn get_space(&self) -> &S {
        &self.space
    }

    /// Get a mutable reference to the space this camera works in
    pub fn get_space_mut(&mut self) -> &mut S {
        &mut self.space
    }

    /// Get the POD uniform struct this camera represents
    pub fn to_uniform(&self, graphics: &Graphics, _et: f32) -> CameraUniform {
        CameraUniform {
            view_proj_mat: self.space.view_proj_mat(graphics).to_cols_array_2d(),
            cam_position: self.space.position().to_array(),
            frame: graphics.canvas.frame_count() as f32
        }
    }
}

/// A camera space that represents a 2D canvas drawing surface.
/// 
/// This provides a rendering space where (0,0) is in the bottom left corner, 
/// and (0, 1) is in the top left corner. 
/// 
/// For non-square canvases, the x-axis is scaled by the aspect ratio.
/// 
/// This is best used for UIs or HUDs.
pub struct ScreenSpace;

impl CameraSpace for ScreenSpace {
    fn label(&self) -> &'static str { "screen_space_camera" }

    fn view_proj_mat(&self, graphics: &Graphics) -> Mat4 {
        let aspect = graphics.canvas.aspect();

        Mat4::orthographic_lh(0.0, aspect, 0.0, 1.0, -1.0, 1.0)
    }

    fn position(&self) -> Vec3 { Vec3::ZERO }
}

/// A camera space with orthogonal projection
pub struct Orthogonal {
    pub transform: Transform,
    
    near: f32,
    far: f32,

    inverted: bool,
}

impl Orthogonal {
    pub fn new(near: f32, far: f32) -> Self {
        Self {
            near, far,
            transform: Transform::default(),
            inverted: false
        }
    }

    /// Set the inversion of the space.
    /// 
    /// When true, this inverts the view-projection matrix. This is most useful for shaders that need to unproject clip space back into the scene, such as with ray tracing.
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }
}

impl Default for Orthogonal {
    fn default() -> Self {
        Orthogonal::new(0.01, 1000.0)
    }
}

impl CameraSpace for Orthogonal {
    fn label(&self) -> &'static str { "orthogonal_camera" }

    fn view_proj_mat(&self, graphics: &Graphics) -> Mat4 {
        let aspect = graphics.canvas.aspect();

        let left = -aspect;
        let right = aspect;
        let bottom = -1.0; 
        let top = 1.0;
        let proj_mat = Mat4::orthographic_lh(left, right, bottom, top, self.near, self.far);
        
        let view_mat = self.transform.as_view_mat();
        let view_proj = proj_mat * view_mat;

        return if self.inverted { view_proj.inverse() } else { view_proj };
    }

    fn position(&self) -> Vec3 { self.transform.get_position() }
}

impl Controllable for Orthogonal {
    fn transform(&self) -> &Transform { &self.transform }

    fn transform_mut(&mut self) -> &mut Transform { &mut self.transform }

    /// Get the camera space's current forward axis
    fn forward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Z).normalize()
    }

    /// Get the camera space's current rightward axis
    fn rightward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::X).normalize()
    }

    /// Get the camera space's current upward axis
    fn upward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Y).normalize()
    }
}

/// A camera space with perspective projection
pub struct Perspective {
    pub transform: Transform,

    fov_rad: f32,
    z_near: f32,
    z_far: f32,

    inverted: bool
}

impl Perspective {
    pub fn new(z_near: f32, z_far: f32, fov_rad: f32) -> Self {
        Self {
            z_near, z_far, fov_rad,
            transform: Transform::default(),
            inverted: false
        }
    }

    /// Set the value of the near clipping plane
    pub fn set_near_clip(&mut self, z_near: f32) {
        self.z_near = z_near;
    }

    pub fn set_far_clip(&mut self, z_far: f32) {
        self.z_far = z_far;
    }

    pub fn set_fov(&mut self, fov_rad: f32) {
        self.fov_rad = fov_rad;
    }

    /// Set the inversion of the space.
    /// 
    /// When true, this inverts the view-projection matrix. This is most useful for shaders that need to unproject clip space back into the scene, such as with ray tracing.
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }
}

impl Default for Perspective {
    fn default() -> Self {
        Perspective::new(0.01, 1000.0, 60.0_f32.to_radians())
    }
}

impl CameraSpace for Perspective {
    fn label(&self) -> &'static str { "perspective_camera" }

    fn view_proj_mat(&self, graphics: &Graphics) -> Mat4 {
        let aspect = graphics.canvas.aspect();
        
        let proj_mat = Mat4::perspective_lh(self.fov_rad, aspect, self.z_near, self.z_far);
              
        let view_mat = self.transform.as_view_mat();
        let view_proj = proj_mat * view_mat;

        return if self.inverted { view_proj.inverse() } else { view_proj };
    }

    fn position(&self) -> Vec3 { self.transform.get_position() }
}

impl Controllable for Perspective {
    fn transform(&self) -> &Transform { &self.transform }

    fn transform_mut(&mut self) -> &mut Transform { &mut self.transform }

    /// Get the camera space's current forward axis
    fn forward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Z).normalize()
    }

    /// Get the camera space's current rightward axis
    fn rightward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::X).normalize()
    }

    /// Get the camera space's current upward axis
    fn upward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Y).normalize()
    }
}

/// A camera space with orthogonal projection.
/// 
/// The view matrix of this space only contains the rotation of the camera's orientation, and the position is excluded.
/// This is useful for camera-relative rendering such as in voxel ray marched scenes.
/// 
/// The space is inverted by default before being sent to the uniform buffer
pub struct RelativeOrthogonal {
    pub transform: Transform,

    near: f32,
    far: f32,

    inverted: bool,
}

impl RelativeOrthogonal {
    pub fn new(near: f32, far: f32) -> Self {
        Self {
            near, far,
            transform: Transform::default(),
            inverted: true
        }
    }

    /// Set the inversion of the space.
    /// 
    /// When true, this inverts the view-projection matrix. This is most useful for shaders that need to unproject clip space back into the scene, such as with ray tracing.
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }
}

impl Default for RelativeOrthogonal {
    fn default() -> Self {
        RelativeOrthogonal::new(0.01, 1000.0)
    }
}

impl CameraSpace for RelativeOrthogonal {
    fn label(&self) -> &'static str { "relative_orthogonal_camera" }

    fn view_proj_mat(&self, graphics: &Graphics) -> Mat4 {
        let aspect = graphics.canvas.aspect();

        let left = -aspect;
        let right = aspect;
        let bottom = -1.0; 
        let top = 1.0;
        let proj_mat = Mat4::orthographic_lh(left, right, bottom, top, self.near, self.far);
        
        let view_mat = Mat4::from_quat(self.transform.get_rotation()).inverse();
        let view_proj = proj_mat * view_mat;

        return if self.inverted { view_proj.inverse() } else { view_proj };
    }

    fn position(&self) -> Vec3 { self.transform.get_position() }
}

impl Controllable for RelativeOrthogonal {
    fn transform(&self) -> &Transform { &self.transform }

    fn transform_mut(&mut self) -> &mut Transform { &mut self.transform }

    /// Get the camera space's current forward axis
    fn forward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Z).normalize()
    }

    /// Get the camera space's current rightward axis
    fn rightward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::X).normalize()
    }

    /// Get the camera space's current upward axis
    fn upward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Y).normalize()
    }
}

/// A camera space with perspective projection.
/// 
/// The view matrix of this space only contains the rotation of the camera's orientation, and the position is excluded.
/// This is useful for camera-relative rendering such as in voxel ray marched scenes.
/// 
/// The space is inverted by default before being sent to the uniform buffer
pub struct RelativePerspective {
    pub transform: Transform,

    fov_rad: f32,
    z_near: f32,
    z_far: f32,

    inverted: bool,
}

impl RelativePerspective {
    pub fn new(z_near: f32, z_far: f32, fov_rad: f32) -> Self {
        Self {
            z_near, z_far, fov_rad,
            transform: Transform::default(),
            inverted: true
        }
    }

    /// Set the inversion of the space.
    /// 
    /// When true, this inverts the view-projection matrix. This is most useful for shaders that need to unproject clip space back into the scene, such as with ray tracing.
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }
}

impl Default for RelativePerspective {
    fn default() -> Self {
        RelativePerspective::new(0.01, 1000.0, 60.0_f32.to_radians())
    }
}

impl CameraSpace for RelativePerspective {
    fn label(&self) -> &'static str { "relative_perspective_camera" }

    fn view_proj_mat(&self, graphics: &Graphics) -> Mat4 {
        let aspect = graphics.canvas.aspect();
        
        let proj_mat = Mat4::perspective_lh(self.fov_rad, aspect, self.z_near, self.z_far);
              
        let view_mat = Mat4::from_quat(self.transform.get_rotation()).inverse();
        let view_proj = proj_mat * view_mat;

        return if self.inverted { view_proj.inverse() } else { view_proj };
    }

    fn position(&self) -> Vec3 { self.transform.get_position() }
}

impl Controllable for RelativePerspective {
    fn transform(&self) -> &Transform { &self.transform }

    fn transform_mut(&mut self) -> &mut Transform { &mut self.transform }

    /// Get the camera space's current forward axis
    fn forward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Z).normalize()
    }

    /// Get the camera space's current rightward axis
    fn rightward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::X).normalize()
    }

    /// Get the camera space's current upward axis
    fn upward_axis(&self) -> Vec3 {
        (self.transform.get_rotation() * Vec3::Y).normalize()
    }
}