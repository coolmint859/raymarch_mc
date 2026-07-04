use crate::{AppEnv, InputEvent, graphics::GpuPass};

/// Represents transitions between screens
pub enum ScreenTransition {
    /// Switch to the provided screen
    SwitchTo(Box<dyn Screen>),
    /// Exit the app
    Exit,
    None,
}

/// Used to represent the current output mode to the window
pub trait Screen {
    /// Initialize the screen state
    fn init(&mut self, env: &mut AppEnv);

    /// Called when a input event was detected by the OS
    fn input_event(&mut self, event: InputEvent);

    /// Called when the window was resized
    fn on_resize(&mut self, _env: &mut AppEnv) {}

    /// Process user input
    fn process_input(&mut self, env: &mut AppEnv, dt: f32) -> ScreenTransition;

    /// Update the screen state
    fn update(&mut self, env: &mut AppEnv, dt: f32);

    /// Render the screen to the window
    fn render(&mut self, env: &mut AppEnv) -> Vec<GpuPass>;
}