use crate::{InputEvent, utils::{KeyboardHandler, MouseHandler}};

/// Zero-Sized type for inidicating an `InputHandler` needs no bindings
#[derive(Debug, Clone, Copy)]
pub enum NoAction {}

/// Wraps the keyboard and mouse
pub struct InputHandler<K, M> {
    pub keyboard: KeyboardHandler<K>,
    pub mouse: MouseHandler<M>
}

impl<K: Clone, M: Clone> InputHandler<K, M> {
    pub fn new() -> Self {
        Self {
            keyboard: KeyboardHandler::new(),
            mouse: MouseHandler::new(),
        }
    }

    /// Process an `InputEvent` by notifying the appropriate handler.
    pub fn process_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key(key_event) => {
                self.keyboard.key_event(key_event)
            },
            InputEvent::MouseButton { state, button } => {
                self.mouse.button_event(state, button);
            },
            InputEvent::MouseMotion { dx, dy } => {
                self.mouse.motion_event(dx, dy);
            }
        }
    }

    /// Clear all pending events.
    pub fn clear_events(&mut self) {
        self.keyboard.clear_events();
        self.mouse.clear_events();
    }

    /// Reset all the bindings for the keyboard and mouse.
    pub fn reset_bindings(&mut self) {
        self.keyboard.reset_bindings();
        self.mouse.reset_bindings();
    }
}