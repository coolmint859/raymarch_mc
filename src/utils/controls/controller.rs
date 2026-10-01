use crate::utils::Transform;

/// Represents entities that can be controlled (i.e. have a Transform)
pub trait Controllable {
    /// Get a reference to the transform on this Controllable
    fn transform(&self) -> &Transform;
    /// Get a mutable reference to the transform on this Controllable
    fn transform_mut(&mut self) -> &mut Transform;

    /// Get the camera's current forward axis
    fn forward_axis(&self) -> glam::Vec3;
    /// Get the camera's current rightward axis
    fn rightward_axis(&self) -> glam::Vec3;
    /// Get the camera's current upward axis
    fn upward_axis(&self) -> glam::Vec3;
}

/// Provides methods to update an entities's orientation in world space
#[derive(Copy, Clone, Debug)]
pub struct EntityController {
    speed: f32,
    sensitivity: f32,
    
    pitch: f32,
    yaw: f32,
}

impl EntityController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            pitch: 0.0,
            yaw: 0.0,
        }
    }

    /// Reset the delta accumulations to default
    pub fn reset_delta(&mut self) {
        self.pitch = 0.0;
        self.yaw = 0.0;
    }

    /// Move a entity using screen space deltas (typically provided by the mouse)
    /// This is done in a FPS Camera movement style
    pub fn rotate_delta(&mut self, entity: &mut impl Controllable, dx: f64, dy: f64) {
        self.yaw += (dx as f32) * self.sensitivity;
        self.pitch += (dy as f32) * self.sensitivity;

        let max_pitch = 89.0f32.to_radians();
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);

        entity.transform_mut().set_rotation_euler(self.pitch, self.yaw, 0.0);
    }

    /// Move the entity along it's positive forward axis
    pub fn move_forward(&self, entity: &mut impl Controllable, dt: f32) {
        let forward = entity.forward_axis();
        let movement = forward * self.speed * dt;

        entity.transform_mut().translate(movement);
    }

    /// Move the entity along it's negative forward axis
    pub fn move_backward(&self, entity: &mut impl Controllable, dt: f32) {
        let backward = -entity.forward_axis();
        let movement = backward * self.speed * dt;

        entity.transform_mut().translate(movement);
    }

    /// Move the entity along it's positive rightward axis
    pub fn strafe_right(&self, entity: &mut impl Controllable, dt: f32) {
        let right = entity.rightward_axis();
        let movement = right * self.speed * dt;

        entity.transform_mut().translate(movement);
    }

    /// Move the entity along it's negative rightward axis
    pub fn strafe_left(&self, entity: &mut impl Controllable, dt: f32) {
        let left = -entity.rightward_axis();
        let movement = left * self.speed * dt;

        entity.transform_mut().translate(movement);
    }

    /// Move the entity along it's positive upward axis
    pub fn move_up(&self, entity: &mut impl Controllable, dt: f32) {
        let up = entity.upward_axis();
        let movement = up * self.speed * dt;

        entity.transform_mut().translate(movement);
    }

    /// Move the entity along it's negative upwards axis
    pub fn move_down(&self, entity: &mut impl Controllable, dt: f32) {
        let down = -entity.upward_axis();
        let movement = down * self.speed * dt;

        entity.transform_mut().translate(movement);
    }
}