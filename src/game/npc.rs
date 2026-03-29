use crate::core::isom::{grid_to_screen, TILE_H};

/// An NPC character that displays dialogue when interacted with.
pub struct Npc {
    pub grid_x: i32,
    pub grid_y: i32,
    pub name: String,
    pub dialogue: Vec<String>,
    current_line: usize,
    pub bubble_timer: f32, // seconds remaining for dialogue bubble
}

impl Npc {
    pub fn new(grid_x: i32, grid_y: i32, name: String, dialogue: Vec<String>) -> Self {
        Self {
            grid_x,
            grid_y,
            name,
            dialogue,
            current_line: 0,
            bubble_timer: 0.0,
        }
    }

    /// Trigger interaction — select dialogue line based on hint_index.
    /// hint_index selects which dialogue to show based on player state.
    /// Returns the dialogue text to display, or None if no dialogue.
    pub fn interact(&mut self, hint_index: usize) -> Option<&str> {
        if self.dialogue.is_empty() {
            return None;
        }
        let idx = hint_index % self.dialogue.len();
        self.current_line = (idx + 1) % self.dialogue.len();
        self.bubble_timer = 2.5; // bubble visible for 2.5 seconds
        Some(&self.dialogue[idx])
    }

    /// Decay the bubble timer. Call each frame with delta time.
    pub fn tick(&mut self, dt: f32) {
        if self.bubble_timer > 0.0 {
            self.bubble_timer -= dt;
        }
    }

    /// Whether the dialogue bubble should be visible.
    pub fn bubble_visible(&self) -> bool {
        self.bubble_timer > 0.0 && !self.dialogue.is_empty()
    }

    /// Get bubble screen position (top-left of bubble) given camera offset.
    pub fn bubble_screen_xy(&self, cam_x: f32, cam_y: f32) -> (i32, i32) {
        let (px, py) = grid_to_screen(self.grid_x as f32, self.grid_y as f32, cam_x, cam_y);
        // Bubble appears above the NPC sprite
        (px as i32, (py - TILE_H - 24.0) as i32)
    }

    /// Get the currently displayed dialogue line (the line shown in the bubble).
    /// Note: after interact() is called, current_line points to the NEXT line,
    /// so this returns current_line - 1 (with wrap-around).
    pub fn current_line_text(&self) -> Option<&str> {
        if self.dialogue.is_empty() {
            return None;
        }
        // current_line already advanced past the displayed line
        let idx = if self.current_line == 0 {
            self.dialogue.len() - 1
        } else {
            self.current_line - 1
        };
        self.dialogue.get(idx).map(|s| s.as_str())
    }

    /// Advance to the next dialogue line (for manual advance via A/Enter/Space).
    /// Only works if dialogue is already visible (bubble_timer > 0).
    /// Returns the dialogue text to display, or None if dialogue ended/dismissed.
    /// When returning None, the bubble should be dismissed.
    pub fn advance(&mut self) -> Option<&str> {
        if self.dialogue.is_empty() {
            return None;
        }
        // Only allow advance if dialogue is already being shown
        if self.bubble_timer <= 0.0 {
            return None;
        }
        // We've shown current line and are advancing to next
        let line = &self.dialogue[self.current_line];
        self.current_line = (self.current_line + 1) % self.dialogue.len();
        // If we wrapped back to 0, we've shown all lines — dismiss bubble
        if self.current_line == 0 {
            self.bubble_timer = 0.0;
            return None;
        }
        // Reset timer for next line (fallback auto-advance)
        self.bubble_timer = 2.5;
        Some(line)
    }
}
