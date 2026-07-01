use glam::*;

pub struct Camera3D {
    eye: glam::Vec3,
    target: glam::Vec3,
    up: glam::Vec3,

    fov_y: f32,
    z_near: f32,
    z_far: f32
}

impl Camera3D {
    pub fn new() -> Self {
        Self {
            eye: Vec3 { x: 0.0, y: 1.5, z: -3.0},
            target: Vec3 { x: 0.0, y: 0.0, z: 0.0},
            up: Vec3 { x: 0.0, y: 1.0, z: 0.0 },

            fov_y: 45.0_f32.to_radians(),
            z_near: 0.01,
            z_far: 100.0
        }
    }

    pub fn set_position(&mut self, pos: Vec3) {
        self.eye = pos;
    }

    /// Get the view-projection matrix for this camera
    pub fn get_view_proj(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_at_lh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_lh(self.fov_y, aspect, self.z_near, self.z_far);
        
        return proj * view;
    }
}