use sdl2::event::Event;
use std::collections::HashMap;

/// Tracks which player a given SDL2 finger_id belongs to.
/// SDL2 finger_ids are arbitrary and reusable, so we must track
/// the mapping ourselves based on which screen half the touch started on.
struct FingerTracker {
    /// Maps SDL2 finger_id -> player number (1 or 2)
    fingers: HashMap<i64, u8>,
}

impl FingerTracker {
    fn new() -> Self {
        Self {
            fingers: HashMap::new(),
        }
    }

    /// Returns the player number (1 or 2) for a given finger_id.
    /// If the finger is not tracked, assigns it based on screen position.
    fn get_player(&mut self, finger_id: i64, x: f32, screen_width: u32) -> u8 {
        let half = screen_width as f32 / 2.0;
        let player = if x < half { 1 } else { 2 };
        self.fingers.insert(finger_id, player);
        player
    }

    /// Remove a finger and return its player, or None if not tracked.
    fn release(&mut self, finger_id: i64) -> Option<u8> {
        self.fingers.remove(&finger_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameInput {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Interact,
    Confirm,
    Back,
    Pause,
}

pub struct VirtualGamepad {
    pub inputs: HashMap<GameInput, bool>,
    pub joystick_x: f32,
    pub joystick_y: f32,
}

impl VirtualGamepad {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            joystick_x: 0.0,
            joystick_y: 0.0,
        }
    }
}

pub struct TouchHandler {
    pub player1: VirtualGamepad,
    pub player2: VirtualGamepad,
    screen_width: u32,
    screen_height: u32,
    finger_tracker: FingerTracker,
}

impl TouchHandler {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            player1: VirtualGamepad::new(),
            player2: VirtualGamepad::new(),
            screen_width,
            screen_height,
            finger_tracker: FingerTracker::new(),
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::FingerDown { x, y, finger_id, .. } => {
                let player = self.finger_tracker.get_player(*finger_id, *x, self.screen_width);
                let half = self.screen_width as f32 / 2.0;
                if player == 1 {
                    self.player1.joystick_x = *x / half * 2.0 - 1.0;
                    self.player1.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                } else {
                    self.player2.joystick_x = (*x - half) / half * 2.0 - 1.0;
                    self.player2.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                }
            }
            Event::FingerUp { finger_id, .. } => {
                if let Some(player) = self.finger_tracker.release(*finger_id) {
                    if player == 1 {
                        self.player1.joystick_x = 0.0;
                        self.player1.joystick_y = 0.0;
                    } else {
                        self.player2.joystick_x = 0.0;
                        self.player2.joystick_y = 0.0;
                    }
                }
            }
            Event::FingerMotion { x, y, finger_id, .. } => {
                // Only update if we know which player this finger belongs to
                if let Some(&player) = self.finger_tracker.fingers.get(finger_id) {
                    let half = self.screen_width as f32 / 2.0;
                    if player == 1 {
                        self.player1.joystick_x = *x / half * 2.0 - 1.0;
                        self.player1.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                    } else {
                        self.player2.joystick_x = (*x - half) / half * 2.0 - 1.0;
                        self.player2.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn joystick_to_directions(&self, pad: &VirtualGamepad) -> Vec<GameInput> {
        let deadzone = 0.3;
        let mut dirs = Vec::new();
        if pad.joystick_x < -deadzone { dirs.push(GameInput::MoveLeft); }
        if pad.joystick_x > deadzone  { dirs.push(GameInput::MoveRight); }
        if pad.joystick_y < -deadzone { dirs.push(GameInput::MoveUp); }
        if pad.joystick_y > deadzone  { dirs.push(GameInput::MoveDown); }
        dirs
    }
}

impl Default for VirtualGamepad {
    fn default() -> Self { Self::new() }
}
