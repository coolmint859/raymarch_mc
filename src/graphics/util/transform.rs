#![allow(dead_code)]
use std::{cell::Cell, sync::atomic::{AtomicU32, Ordering}};

use glam::*;

static TRANSFORM_COUNTER: AtomicU32 = AtomicU32::new(0);

/// represents position, rotation, and scaling of an entity
#[derive(Clone, Debug)]
pub struct Transform {
    id: u32,
    position: Vec3,
    rotation: Quat,
    scale: Vec3,

    world_mat: Cell<Mat4>,
}

impl Transform {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let id = TRANSFORM_COUNTER.fetch_add(1, Ordering::SeqCst);

        let world_mat = Cell::new(Mat4::from_scale_rotation_translation(scale, rotation, position));
        Self { id, position, rotation, scale, world_mat }
    }

    /// Set the postition of the transform relative to the world axis
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    /// Set the scale of the transform
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Set the rotation of the transform relative to the local center
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn id(&self) -> u32 {
        self.id.clone()
    }

    /// Get the position of this transform
    pub fn get_position(&self) -> Vec3 {
        self.position.clone()
    }

    /// Get the rotation of this transform
    pub fn get_rotation(&self) -> Quat {
        self.rotation.clone()
    }

    /// Get the scale of this transform
    pub fn get_scale(&self) -> Vec3 {
        self.scale.clone()
    }

    /// Move relative to local origin
    pub fn translate(&mut self, amount: Vec3) {
        self.position += amount;
    }

    /// Move relative to world origin
    pub fn move_to(&mut self, position: Vec3) {
        self.position = position;
    }

    /// Set the x value for this transform relative to the world origin
    pub fn set_x(&mut self, x: f32) {
        self.position.x = x;
    }

    /// Set the y value for this transform relative to the world origin
    pub fn set_y(&mut self, y: f32) {
        self.position.y = y;
    }

    /// Set the z value for this transform relative to the world origin
    pub fn set_z(&mut self, z: f32) {
        self.position.z = z;
    }

    /// Rotate from current orientation
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
    }

    /// Rotate from current orientation, using Euler angles
    pub fn rotate_euler(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.rotation *= Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }

    /// Set the absolute rotation of the transform
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
    }

    /// Set the absolute rotation of the transform using Euler angles
    pub fn set_rotation_euler(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }

    /// Reorient this transform to 'point' to a target
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let look_dir = self.position - target;
        self.rotation = Quat::from_mat4(&Mat4::look_at_rh(self.position, look_dir, up.normalize()));
    }

    /// Set the scale of this transform
    pub fn set_scale(&mut self, scale: glam::Vec3) {
        self.scale = scale;
    }

    /// Apply this transform to a vector
    pub fn apply_to(&self, vector:Vec3) -> Vec3 {
        let vec4 = Vec4::new(vector.x, vector.y, vector.z, 1.0);
        let transformed = self.world_mat.get().mul_vec4(vec4);
        transformed.xyz()
    }

    /// Update and get a copy of this transform's world matrix
    pub fn to_updated(&self) -> glam::Mat4 {
        self.world_mat.set(Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position));
        self.world_mat.get()
    }

    /// Get the size in bytes of a transform instance
    pub fn size() -> usize {
        return std::mem::size_of::<glam::Mat4>()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE)
    }
}