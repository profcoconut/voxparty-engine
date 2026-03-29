use sdl2::event::Event;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::collections::HashMap;
use crate::core::isom::screen_to_grid;

/// Tap tracking state for a single finger.
struct TapState {
    finger_id: i64,
    start_x: f32,
    start_y: f32,
    start_time: u32,
}

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
    /// Queued tap-to-move target in screen coordinates (sx, sy).
    /// Converted to grid coords in take_pending_move using current camera.
    pending_screen: Option<(f32, f32)>,
    /// Active tap tracking (finger_id -> TapState)
    active_taps: HashMap<i64, TapState>,
}

impl TouchHandler {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            player1: VirtualGamepad::new(),
            player2: VirtualGamepad::new(),
            screen_width,
            screen_height,
            finger_tracker: FingerTracker::new(),
            pending_screen: None,
            active_taps: HashMap::new(),
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::FingerDown { x, y, finger_id, timestamp, .. } => {
                let player = self.finger_tracker.get_player(*finger_id, *x, self.screen_width);
                let half = self.screen_width as f32 / 2.0;
                if player == 1 {
                    self.player1.joystick_x = *x / half * 2.0 - 1.0;
                    self.player1.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                } else {
                    self.player2.joystick_x = (*x - half) / half * 2.0 - 1.0;
                    self.player2.joystick_y = *y / self.screen_height as f32 * 2.0 - 1.0;
                }
                // Start tracking tap for tap-to-move detection (player 1 only)
                if player == 1 {
                    let screen_x = *x * self.screen_width as f32;
                    let screen_y = *y * self.screen_height as f32;
                    self.active_taps.insert(*finger_id, TapState {
                        finger_id: *finger_id,
                        start_x: screen_x,
                        start_y: screen_y,
                        start_time: *timestamp,
                    });
                }
            }
            Event::FingerUp { finger_id, x, y, timestamp, .. } => {
                let player = self.finger_tracker.release(*finger_id);
                if let Some(p) = player {
                    if p == 1 {
                        self.player1.joystick_x = 0.0;
                        self.player1.joystick_y = 0.0;
                    } else {
                        self.player2.joystick_x = 0.0;
                        self.player2.joystick_y = 0.0;
                    }
                }
                // Check if this was a tap (player 1 only)
                if let Some(tap) = self.active_taps.remove(finger_id) {
                    if tap.finger_id == *finger_id {
                        // Tap detection: within 200ms and minimal movement
                        let tap_duration = timestamp.saturating_sub(tap.start_time);
                        let max_tap_duration: u32 = 200_000; // 200ms in microseconds
                        let screen_x = *x * self.screen_width as f32;
                        let screen_y = *y * self.screen_height as f32;
                        let dx = screen_x - tap.start_x;
                        let dy = screen_y - tap.start_y;
                        let move_threshold = 20.0f32; // 20 pixels = tap vs drag
                        if tap_duration <= max_tap_duration
                            && dx.abs() < move_threshold
                            && dy.abs() < move_threshold
                        {
                            // It's a tap! Store raw screen coords for later grid conversion
                            self.pending_screen = Some((screen_x, screen_y));
                        }
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

    /// Consumes any pending tap-to-move target and returns it as grid coordinates.
    /// Converts stored screen coords to grid coords using the provided camera position.
    /// Returns None if no pending move is queued.
    pub fn take_pending_move(&mut self, cam_x: f32, cam_y: f32) -> Option<(i32, i32)> {
        if let Some((sx, sy)) = self.pending_screen.take() {
            let (gx, gy) = screen_to_grid(sx, sy, cam_x, cam_y);
            return Some((gx.round() as i32, gy.round() as i32));
        }
        None
    }

    /// Renders a visual virtual gamepad overlay on the bottom of the screen.
    /// D-pad on LEFT (player 1), action button on RIGHT.
    /// Uses pixel-art style blocky shapes with semi-transparent colors.
    pub fn render(&self, canvas: &mut Canvas<Window>) {
        let screen_w = self.screen_width;
        let screen_h = self.screen_height;

        // Bottom 200px of screen for the gamepad overlay
        let overlay_bottom = screen_h as i32;
        let overlay_top = (screen_h as i32 - 200).max(0);

        // D-pad dimensions (pixel-art style, blocky)
        let dpad_size = 80i32;       // Total width/height of d-pad cross
        let dpad_arm = 24i32;        // Width of each arm
        let dpad_center = 50i32;     // Size of center square
        let dpad_total = dpad_arm * 2 + dpad_center; // 72px total (close to 80)

        // D-pad position: left side, bottom of screen, centered in bottom 200px
        let dpad_x = 60i32;
        let dpad_y = overlay_top + (200 - dpad_total) as i32 / 2;

        // Action button position: right side of screen
        let btn_radius = 25i32;
        let btn_x = (screen_w as i32 - 100);
        let btn_y = overlay_top + 100;

        // Colors (semi-transparent for game visibility)
        let color_inactive = sdl2::pixels::Color::RGBA(80, 80, 80, 160);
        let color_active = sdl2::pixels::Color::RGBA(255, 200, 50, 220);
        let color_bg = sdl2::pixels::Color::RGBA(40, 40, 40, 180);

        // Helper to get direction active state for player 1
        let jx = self.player1.joystick_x;
        let jy = self.player1.joystick_y;
        let deadzone = 0.3;
        let active_left = jx < -deadzone;
        let active_right = jx > deadzone;
        let active_up = jy < -deadzone;
        let active_down = jy > deadzone;

        // Draw semi-transparent background bar
        canvas.set_draw_color(color_bg);
        let _ = canvas.fill_rect(Rect::new(0, overlay_top, screen_w, 200));

        // === D-PAD (Player 1, LEFT side) ===
        // Center square
        let center_rect = Rect::new(
            dpad_x + dpad_arm,
            dpad_y + dpad_arm,
            dpad_center as u32,
            dpad_center as u32,
        );
        canvas.set_draw_color(color_inactive);
        let _ = canvas.fill_rect(center_rect);

        // UP arm (top)
        let up_rect = Rect::new(
            dpad_x + dpad_arm,
            dpad_y,
            dpad_center as u32,
            dpad_arm as u32,
        );
        canvas.set_draw_color(if active_up { color_active } else { color_inactive });
        let _ = canvas.fill_rect(up_rect);

        // DOWN arm (bottom)
        let down_rect = Rect::new(
            dpad_x + dpad_arm,
            dpad_y + dpad_arm + dpad_center,
            dpad_center as u32,
            dpad_arm as u32,
        );
        canvas.set_draw_color(if active_down { color_active } else { color_inactive });
        let _ = canvas.fill_rect(down_rect);

        // LEFT arm
        let left_rect = Rect::new(
            dpad_x,
            dpad_y + dpad_arm,
            dpad_arm as u32,
            dpad_center as u32,
        );
        canvas.set_draw_color(if active_left { color_active } else { color_inactive });
        let _ = canvas.fill_rect(left_rect);

        // RIGHT arm
        let right_rect = Rect::new(
            dpad_x + dpad_arm + dpad_center,
            dpad_y + dpad_arm,
            dpad_arm as u32,
            dpad_center as u32,
        );
        canvas.set_draw_color(if active_right { color_active } else { color_inactive });
        let _ = canvas.fill_rect(right_rect);

        // === ACTION BUTTON (A) on RIGHT side ===
        // Draw a pixel-art circle approximation using filled rects
        let btn_color = if self.player1.inputs.get(&GameInput::Jump).copied().unwrap_or(false)
            || self.player1.inputs.get(&GameInput::Confirm).copied().unwrap_or(false)
        {
            color_active
        } else {
            color_inactive
        };

        canvas.set_draw_color(btn_color);

        // Pixel-art circle: approximate with corner squares and cross
        // Center
        let _ = canvas.fill_rect(Rect::new(btn_x - btn_radius/2, btn_y - btn_radius/2, btn_radius as u32, btn_radius as u32));

        // Top-left to bottom-right diagonal strips (pixel-art circle)
        let strip_size = 6i32;
        for i in 0..4 {
            let offset = (i as i32 - 1) * strip_size;
            // Horizontal strips
            let _ = canvas.fill_rect(Rect::new(
                btn_x - btn_radius + offset,
                btn_y - strip_size/2,
                (btn_radius * 2 - offset.abs() as i32 * 2) as u32,
                strip_size as u32,
            ));
        }
    }
}

impl Default for VirtualGamepad {
    fn default() -> Self { Self::new() }
}
