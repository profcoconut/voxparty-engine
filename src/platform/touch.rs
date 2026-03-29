use sdl2::event::Event;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameInput {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Interact,
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
}

impl TouchHandler {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            player1: VirtualGamepad::new(),
            player2: VirtualGamepad::new(),
            screen_width,
            screen_height,
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::FingerDown { x, y, finger_id: _, .. } => {
                let half = self.screen_width as f32 / 2.0;
                if *x < half {
                    self.player1.joystick_x = *x / half * 2.0 - 1.0;
                    self.player1.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                } else {
                    self.player2.joystick_x = (*x - half) / half * 2.0 - 1.0;
                    self.player2.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                }
            }
            Event::FingerUp { finger_id, .. } => {
                if *finger_id < 2 {
                    self.player1.joystick_x = 0.0;
                    self.player1.joystick_y = 0.0;
                } else {
                    self.player2.joystick_x = 0.0;
                    self.player2.joystick_y = 0.0;
                }
            }
            Event::FingerMotion { x, y, finger_id, .. } => {
                let half = self.screen_width as f32 / 2.0;
                if *finger_id < 2 {
                    self.player1.joystick_x = *x / half * 2.0 - 1.0;
                    self.player1.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                } else {
                    self.player2.joystick_x = (*x - half) / half * 2.0 - 1.0;
                    self.player2.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
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
