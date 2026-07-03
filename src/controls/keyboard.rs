use winit::{
    event::{ElementState, KeyEvent}, 
    keyboard::{KeyCode, PhysicalKey},
};
use std::{
    collections::{ HashMap, HashSet }, 
    mem
};

pub struct KeyboardHandler<K> {
    bindings: HashMap<KeyCode, K>,

    raw_held_keys: HashSet<KeyCode>,
    just_pressed: Vec<K>,
    just_released: Vec<K>,
    held_keys: Vec<K>
}

impl<K: Clone> KeyboardHandler<K> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            raw_held_keys: HashSet::new(),
            just_pressed: Vec::new(),
            just_released: Vec::new(),
            held_keys: Vec::new()
        }
    }

    /// Map a key code to a variant on the enum T
    pub fn register_key(&mut self, key: KeyCode, action: K) {
        self.bindings.insert(key, action);
    }

    /// Mark a key event as detected. Should be called for each Keyboard OS event.
    pub fn key_event(&mut self, key_event: &KeyEvent) {
        if key_event.repeat { return; }

        let key_code = match key_event.physical_key {
            PhysicalKey::Code(code) => code,
            PhysicalKey::Unidentified(_) => { return; }
        };

        if let Some(action) = self.bindings.get(&key_code) {
            if key_event.state == ElementState::Pressed {
                if !self.raw_held_keys.contains(&key_code) {
                    self.raw_held_keys.insert(key_code);
                    self.just_pressed.push(action.clone())
                }
            } else {
                self.raw_held_keys.remove(&key_code);
                self.just_released.push(action.clone())
            }
        }
    }

    /// Poll key press events that occurred since the last poll
    pub fn poll_on_press(&mut self) -> std::vec::IntoIter<K> {
        mem::take(&mut self.just_pressed).into_iter()
    }

    /// Poll key release events that occurred since the last poll
    pub fn poll_on_release(&mut self) -> std::vec::IntoIter<K> {
        mem::take(&mut self.just_released).into_iter()
    }

    /// Poll key hold events that are still occurring this frame
    pub fn poll_on_held(&mut self) -> std::vec::IntoIter<K> {
        self.held_keys.clear();

        for key_code in &self.raw_held_keys {
            if let Some(action) = self.bindings.get(key_code) {
                self.held_keys.push(action.clone())
            }
        }

        mem::take(&mut self.held_keys).into_iter()
    }

    /// Peek at key press events non-destructively
    pub fn peek_on_press(&self) -> impl IntoIterator<Item = &K> {
        self.just_pressed.iter()
    }

    /// Peek at key release events non-destructively
    pub fn peek_on_release(&self) -> impl IntoIterator<Item = &K> {
        self.just_released.iter()
    }

    /// Peek at key hold events non-destructively
    pub fn peek_on_held(&mut self) -> impl IntoIterator<Item = &K> {
        self.held_keys.clear();

        for key_code in &self.raw_held_keys {
            if let Some(action) = self.bindings.get(key_code) {
                self.held_keys.push(action.clone())
            }
        }
        
        self.held_keys.iter()
    }

    /// Clear the detected key events since the last poll
    pub fn clear_events(&mut self) {
        self.held_keys.clear();
        self.just_pressed.clear();
        self.just_released.clear();
    }
}