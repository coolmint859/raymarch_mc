use std::collections::{HashMap, HashSet};

use winit::event::{ElementState, MouseButton};

#[derive(Clone, Copy)]
pub struct DeltaMotion {
    pub dx: f64,
    pub dy: f64,
}

pub struct MouseHandler<M> {
    bindings: HashMap<MouseButton, M>,

    raw_held_buttons: HashSet<MouseButton>,

    just_pressed: Vec<M>,
    just_released: Vec<M>,
    held_buttons: Vec<M>,

    delta_motion: DeltaMotion
}

impl<M: Clone> MouseHandler<M> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            raw_held_buttons: HashSet::new(),
            just_pressed: Vec::new(),
            just_released: Vec::new(),
            held_buttons: Vec::new(),

            delta_motion: DeltaMotion { dx: 0.0, dy: 0.0 }
        }
    }

    /// map a mouse button event to an action M
    pub fn register_button(&mut self, button: MouseButton, action: M) {
        self.bindings.insert(button, action);
    }

    /// Mark a mouse button event as detected. Should be called for each Mouse OS event.
    pub fn button_event(&mut self, state: ElementState, button: MouseButton) {
        if let Some(action) = self.bindings.get(&button) {
            if state == ElementState::Pressed {
                if !self.raw_held_buttons.contains(&button) {
                    self.raw_held_buttons.insert(button);
                    self.just_pressed.push(action.clone())
                }
            } else {
                self.raw_held_buttons.remove(&button);
                self.just_released.push(action.clone())
            }
        }
    }

    /// Mark a mouse motion event as detected. Should be called for each Keyboard OS event.
    pub fn motion_event(&mut self, dx: f64, dy: f64) {
        self.delta_motion.dx += dx;
        self.delta_motion.dy += dy;
    }

    /// Poll mouse button press events that occurred since the last poll
    pub fn poll_on_press(&mut self) -> std::vec::IntoIter<M> {
        std::mem::take(&mut self.just_pressed).into_iter()
    }

    /// Poll mouse button release events that occurred since the last poll
    pub fn poll_on_release(&mut self) -> std::vec::IntoIter<M> {
        std::mem::take(&mut self.just_released).into_iter()
    }

    /// Poll mouse button hold events that occurred since the last poll
    pub fn poll_on_held(&mut self) -> std::vec::IntoIter<M> {
        self.held_buttons.clear();

        for button in &self.raw_held_buttons {
            if let Some(action) = self.bindings.get(button) {
                self.held_buttons.push(action.clone());
            }
        }

        std::mem::take(&mut self.held_buttons).into_iter()
    }

    /// Poll mouse motion events that occurred since the last poll
    pub fn poll_motion(&mut self) -> DeltaMotion {
        let motion = self.delta_motion.clone();

        self.delta_motion.dx = 0.0;
        self.delta_motion.dy = 0.0;

        motion
    }

    /// Clear the detected mouse events since the last poll
    pub fn clear_events(&mut self) {
        self.held_buttons.clear();
        self.just_pressed.clear();
        self.just_released.clear();
    }
}