use crate::graphics::PerspectiveCamera;

/// Provides methods to update a camera's orientation
pub struct CameraController {
    speed: f32,
    sensitivity: f32,
    
    pitch: f32,
    yaw: f32,
}

impl CameraController {
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

    /// Move a camera using screen space deltas (typically provided by the mouse)
    /// This is done in a FPS Camera movement style
    pub fn rotate_delta(&mut self, camera: &mut PerspectiveCamera, dx: f64, dy: f64) {
        self.yaw += (dx as f32) * self.sensitivity;
        self.pitch += (dy as f32) * self.sensitivity;

        // println!("dx: {}, dy: {}, yaw: {}, pitch: {}", dx, dy, self.yaw, self.pitch);

        let max_pitch = 89.0f32.to_radians();
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);

        camera.transform.set_rotation_euler(self.pitch, self.yaw, 0.0);
    }

    /// Move the camera along it's positive forward axis
    pub fn move_forward(&self, camera: &mut PerspectiveCamera, dt: f32) {
        let forward = camera.forward_axis();
        let movement = forward * self.speed * dt;

        // println!("Moving forward {:?}", movement);

        camera.transform.translate(movement);
    }

    /// Move the camera along it's negative forward axis
    pub fn move_backward(&self, camera: &mut PerspectiveCamera, dt: f32) {
        let backward = -camera.forward_axis();
        let movement = backward * self.speed * dt;

        // println!("Moving backward {:?}", movement);

        camera.transform.translate(movement);
    }

    /// Move the camera along it's positive rightward axis
    pub fn strafe_right(&self, camera: &mut PerspectiveCamera, dt: f32) {
        let right = camera.rightward_axis();
        let movement = right * self.speed * dt;

        // println!("Moving right {:?}", movement);

        camera.transform.translate(movement);
    }

    /// Move the camera along it's negative rightward axis
    pub fn strafe_left(&self, camera: &mut PerspectiveCamera, dt: f32) {
        let left = -camera.rightward_axis();
        let movement = left * self.speed * dt;

        // println!("Moving left {:?}", movement);

        camera.transform.translate(movement);
    }

    /// Move the camera along it's positive upward axis
    pub fn move_up(&self, camera: &mut PerspectiveCamera, dt: f32) {
        let up = camera.upward_axis();
        let movement = up * self.speed * dt;

        // println!("Moving up {:?}", movement);

        camera.transform.translate(movement);
    }

    /// Move the camera along it's negative upwards axis
    pub fn move_down(&self, camera: &mut PerspectiveCamera, dt: f32) {
        let down = -camera.upward_axis();
        let movement = down * self.speed * dt;

        // println!("Moving down {:?}", movement);

        camera.transform.translate(movement);
    }
}