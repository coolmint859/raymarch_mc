use glam::*;

use crate::utils::Transform;

/// A camera that embodies perspective projection
pub struct PerspectiveCamera {
    pub transform: Transform,

    fov_y: f32,
    z_near: f32,
    z_far: f32
}

impl PerspectiveCamera {
    pub fn new() -> Self {
        Self {
            transform: Transform::default(),
            fov_y: 60.0_f32.to_radians(),
            z_near: 0.01,
            z_far: 100.0
        }
    }

    /// Get the view-projection matrix for this camera
    pub fn get_view_proj(&self, aspect: f32) -> Mat4 {
        let view_mat = self.transform.to_updated().inverse();
        let proj_mat = Mat4::perspective_lh(self.fov_y, aspect, self.z_near, self.z_far);
        
        return proj_mat * view_mat;
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