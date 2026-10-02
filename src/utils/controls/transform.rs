#![allow(dead_code)]
use std::{sync::atomic::{AtomicU32, Ordering}};

use glam::*;

use crate::graphics::Serializable;

static TRANSFORM_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Represents the translation, rotation, and scaling of an entity
/// 
/// Implements Serializable
#[derive(Clone, Debug)]
pub struct Transform {
    id: u32,
    position: Vec3,
    rotation: Quat,
    scale: Vec3,

    world_mat: Mat4,
}

impl Transform {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let id = TRANSFORM_COUNTER.fetch_add(1, Ordering::SeqCst);

        let world_mat = Mat4::from_scale_rotation_translation(scale, rotation, position);
        Self { id, position, rotation, scale, world_mat }
    }

    /// Create a new transform with an initial position
    pub fn from_position(pos: Vec3) -> Self {
        Transform::default().with_position(pos)
    }

    /// Create a new transform with an initial rotation
    pub fn from_rotation(rot: Quat) -> Self {
        Transform::default().with_rotation(rot)
    }

    /// Create a new transform with an initial scale
    pub fn from_scale(scale: Vec3) -> Self {
        Transform::default().with_scale(scale)
    }
    
    /// Set the postition of the transform relative to the world axis
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;

        self.recalc_mat();
        self
    }

    /// Set the scale of the transform
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;

        self.recalc_mat();
        self
    }

    /// Set the rotation of the transform relative to the local center
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;

        self.recalc_mat();
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

        self.recalc_mat();
    }

    /// Move relative to world origin
    pub fn move_to(&mut self, position: Vec3) {
        self.position = position;

        self.recalc_mat();
    }

    /// Set the x value for this transform relative to the world origin
    pub fn set_x(&mut self, x: f32) {
        self.position.x = x;

        self.recalc_mat();
    }

    /// Set the y value for this transform relative to the world origin
    pub fn set_y(&mut self, y: f32) {
        self.position.y = y;

        self.recalc_mat();
    }

    /// Set the z value for this transform relative to the world origin
    pub fn set_z(&mut self, z: f32) {
        self.position.z = z;

        self.recalc_mat();
    }

    /// Rotate from current orientation
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;

        self.recalc_mat();
    }

    /// Rotate from current orientation, using Euler angles
    pub fn rotate_euler(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.rotation *= Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

        self.recalc_mat();
    }

    /// Set the absolute rotation of the transform
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;

        self.recalc_mat();
    }

    /// Set the absolute rotation of the transform using Euler angles
    pub fn set_rotation_euler(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

        self.recalc_mat();
    }

    /// Reorient this transform to 'point' to a target
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let look_dir = self.position - target;
        self.rotation = Quat::from_mat4(&Mat4::look_at_rh(self.position, look_dir, up.normalize()));

        self.recalc_mat();
    }

    /// Set the scale of this transform
    pub fn set_scale(&mut self, scale: glam::Vec3) {
        self.scale = scale;

        self.recalc_mat();
    }

    /// Apply this transform to a vector
    pub fn apply_to(&self, vector:Vec3) -> Vec3 {
        let vec4 = Vec4::new(vector.x, vector.y, vector.z, 1.0);
        let transformed = self.world_mat.mul_vec4(vec4);
        transformed.xyz()
    }

    /// Update and get a copy of this transform's world matrix
    // pub fn to_updated(&self) -> glam::Mat4 {
    //     self.world_mat.set(Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position));
    //     self.world_mat.get()
    // }

    /// Convert this transform into a column-oriented array `[f32; 16]`
    pub fn to_cols_array(&self) -> [f32; 16] {
        self.world_mat.to_cols_array()
    }

    /// Convert this transform into a row-oriented array `[f32; 16]`
    pub fn to_rows_array(&self) -> [f32; 16] {
        self.world_mat.transpose().to_cols_array()
    }

    /// Get this transform as a view matrix (for camera systems)
    pub fn as_view_mat(&self) -> Mat4 {
        self.world_mat.inverse()
    }

    /// Get this transform as a world/model matrix (for camera systems)
    pub fn as_world_mat(&self) -> Mat4 {
        self.world_mat
    }

    /// Get the size in bytes of a transform instance
    pub fn size() -> usize {
        return std::mem::size_of::<glam::Mat4>()
    }

    /// Recalculate the inner world matrix of the transform
    fn recalc_mat(&mut self) {
        self.world_mat = Mat4::from_scale_rotation_translation(
            self.scale, 
            self.rotation, 
            self.position
        );
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE)
    }
}

impl Serializable for Transform {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(&self.world_mat)
    }
}