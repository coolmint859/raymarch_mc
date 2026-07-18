use glam::*;

use crate::{utils::Transform};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    inv_view_proj: [[f32; 4]; 4],
    camera_postion: [f32; 3],
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
    pub fn to_uniform(&self) -> CameraUniform {
        CameraUniform {
            inv_view_proj: self.view_proj.inverse().to_cols_array_2d(),
            camera_postion: self.transform.get_position().to_array()
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