//! Debug overlay — toggled by F1, renders engine internals via SDL2 primitives.
//! Zero external font dependencies; uses a hardcoded 5x7 bitmap font.

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use super::{depth_key, screen_to_grid, Scene, SceneState};
use crate::core::Camera;
use crate::game::Player;

/// Maximum text line length in characters.
const MAX_CHARS: usize = 40;
/// Character cell dimensions.
const CHAR_W: i32 = 5;
const CHAR_H: i32 = 7;
/// Overlay padding.
const PAD: i32 = 8;

/// Current input directions for display.
#[derive(Default)]
pub struct DebugInputState {
    pub p1_dirs: String,
    pub p2_dirs: String,
}

/// Runtime state read from the game each frame.
pub struct DebugState<'a> {
    pub scene: &'a Scene,
    pub player1: &'a Player,
    pub player2: &'a Player,
    pub camera: &'a Camera,
    pub mouse_screen: (i32, i32),
    pub input: &'a DebugInputState,
    pub god_mode: bool,
}

pub struct DebugOverlay {
    visible: bool,
    fps_visible: bool,
    fps: f32,
    frame_time_ms: f32,
    mouse_grid_x: i32,
    mouse_grid_y: i32,
    mouse_depth: i32,
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {
            visible: false,
            fps_visible: false,
            fps: 0.0,
            frame_time_ms: 0.0,
            mouse_grid_x: 0,
            mouse_grid_y: 0,
            mouse_depth: 0,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn toggle_fps(&mut self) {
        self.fps_visible = !self.fps_visible;
    }

    pub fn is_fps_visible(&self) -> bool {
        self.fps_visible
    }

    /// Update FPS using exponential moving average. Call each frame.
    pub fn update_fps(&mut self, dt: f32) {
        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        // EMA with alpha = 0.1 for smooth display
        self.fps += 0.1 * (fps - self.fps);
        self.frame_time_ms += 0.1 * (dt * 1000.0 - self.frame_time_ms);
    }

    /// Update mouse hover tile from current mouse screen position + camera.
    pub fn update_mouse(&mut self, mouse_screen: (i32, i32), camera: &Camera) {
        let (gx, gy) = screen_to_grid(
            mouse_screen.0 as f32,
            mouse_screen.1 as f32,
            camera.x,
            camera.y,
        );
        self.mouse_grid_x = gx as i32;
        self.mouse_grid_y = gy as i32;
        self.mouse_depth = depth_key(gx as i32, gy as i32, 0);
    }

    /// Render the overlay if visible. Call after canvas.present().
    pub fn render(&self, canvas: &mut Canvas<Window>, state: &DebugState) {
        // Always render FPS counter if fps_visible is true
        if self.fps_visible && !self.visible {
            let (screen_w, _screen_h) = canvas.output_size().unwrap_or((1280, 720));
            // Small FPS badge in top-right corner
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 160));
            let _ = canvas.fill_rect(Rect::new(screen_w as i32 - 110, 8, 102, 24));
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(100, 200, 100, 255));
            let _ = canvas.draw_rect(Rect::new(screen_w as i32 - 110, 8, 102, 24));
            let fps_text = format!("FPS: {:4.1}", self.fps);
            Self::draw_text(
                canvas,
                &fps_text,
                screen_w as i32 - 106,
                14,
                sdl2::pixels::Color::RGBA(180, 255, 180, 255),
            );
        }

        if !self.visible {
            return;
        }

        let (screen_w, _screen_h) = canvas.output_size().unwrap_or((1280, 720));

        // Background panel
        let panel_h = 7 * (CHAR_H + 2) + PAD * 2;
        let panel_w = MAX_CHARS as i32 * CHAR_W + PAD * 2;
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 180));
        let _ = canvas.fill_rect(Rect::new(0, 0, panel_w as u32, panel_h as u32));

        // Border
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(100, 200, 100, 255));
        let _ = canvas.draw_rect(Rect::new(0, 0, panel_w as u32, panel_h as u32));

        let text_color = sdl2::pixels::Color::RGBA(180, 255, 180, 255);
        let x = PAD;
        let mut y = PAD;

        Self::draw_text(canvas, &Self::fps_line(self), x, y, text_color);
        y += CHAR_H + 2;
        Self::draw_text(canvas, &Self::scene_line(state.scene), x, y, text_color);
        y += CHAR_H + 2;
        Self::draw_text(canvas, &Self::player_line(state.player1, 1), x, y, text_color);
        y += CHAR_H + 2;
        Self::draw_text(canvas, &Self::player_line(state.player2, 2), x, y, text_color);
        y += CHAR_H + 2;
        Self::draw_text(canvas, &Self::camera_line(state.camera), x, y, text_color);
        y += CHAR_H + 2;
        Self::draw_text(
            canvas,
            &Self::hover_line(self.mouse_grid_x, self.mouse_grid_y, self.mouse_depth),
            x,
            y,
            text_color,
        );
        y += CHAR_H + 2;
        Self::draw_text(
            canvas,
            &Self::input_line(state),
            x,
            y,
            text_color,
        );

        // God mode indicator
        if state.god_mode {
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 200, 0, 255));
            let _ = canvas.fill_rect(Rect::new(screen_w as i32 - 80, 8, 72, 20));
            Self::draw_text(canvas, "GOD MODE", screen_w as i32 - 76, 12, sdl2::pixels::Color::RGBA(0, 0, 0, 255));
        }
    }

    fn fps_line(&self) -> String {
        format!("FPS: {:4.1}  FT: {:4.1}ms", self.fps, self.frame_time_ms)
    }

    fn scene_line(scene: &Scene) -> String {
        let name = match scene.state {
            SceneState::Menu => "Menu",
            SceneState::EpisodeSelect => "EpisodeSelect",
            SceneState::TitleCard => "TitleCard",
            SceneState::Playing => "Playing",
            SceneState::Victory => "Victory",
            SceneState::GameOver => "GameOver",
            SceneState::Paused => "Paused",
        };
        format!("SCENE: {}", name)
    }

    fn player_line(player: &Player, id: u8) -> String {
        use crate::game::PlayerState;
        let state_str = match player.state {
            PlayerState::Idle => "Idle",
            PlayerState::Moving => "Moving",
            PlayerState::Eliminated => "Elim",
            PlayerState::Won => "Won",
        };
        format!("P{}({}, {}) {}", id, player.grid_x, player.grid_y, state_str)
    }

    fn camera_line(camera: &Camera) -> String {
        format!(
            "CAM tgt=({:5.1},{:5.1}) cur=({:5.1},{:5.1})",
            camera.target_x, camera.target_y, camera.x, camera.y
        )
    }

    fn hover_line(gx: i32, gy: i32, depth: i32) -> String {
        format!("HOVER ({:4}, {:4}) depth={:6}", gx, gy, depth)
    }

    fn input_line(state: &DebugState) -> String {
        format!("P1:[{}] P2:[{}]", state.input.p1_dirs, state.input.p2_dirs)
    }

    // ── 5x7 Bitmap Font ────────────────────────────────────────────────────────

    /// Draw null-terminated ASCII string at pixel position (x, y).
    pub fn draw_text(canvas: &mut Canvas<Window>, s: &str, x: i32, y: i32, color: sdl2::pixels::Color) {
        let mut px = x;
        for ch in s.chars() {
            if px + CHAR_W > 1280 {
                break;
            }
            Self::draw_char(canvas, ch, px, y, color);
            px += CHAR_W + 1;
        }
    }

    pub fn draw_char(canvas: &mut Canvas<Window>, ch: char, x: i32, y: i32, color: sdl2::pixels::Color) {
        let bitmap = font_5x7(ch);
        canvas.set_draw_color(color);
        for row in 0..7 {
            let bits = bitmap[row];
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 != 0 {
                    let _ = canvas.fill_rect(Rect::new(x + col as i32, y + row as i32, 1, 1));
                }
            }
        }
    }

    /// Return 7-byte bitmap for a character (used by tests).
    #[allow(dead_code)]
    fn bitmap_for(ch: char) -> [u8; 7] {
        font_5x7(ch)
    }
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}

// ── Bitmap font table ────────────────────────────────────────────────────────
// Each row is a 5-bit bitmask (bits 4..0 = left..right).
// Characters: space, digits 0-9, A-Z, a-z, common punctuation.

fn font_5x7(ch: char) -> [u8; 7] {
    match ch {
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
        '3' => [0x0E, 0x11, 0x01, 0x06, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0E, 0x11, 0x10, 0x0E, 0x01, 0x11, 0x0E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        'a' => [0x00, 0x00, 0x0E, 0x01, 0x0F, 0x11, 0x0F],
        'b' => [0x10, 0x10, 0x12, 0x1E, 0x11, 0x11, 0x1E],
        'c' => [0x00, 0x00, 0x0E, 0x10, 0x10, 0x11, 0x0E],
        'd' => [0x01, 0x01, 0x0D, 0x13, 0x11, 0x11, 0x0F],
        'e' => [0x00, 0x00, 0x0E, 0x11, 0x1F, 0x10, 0x0E],
        'f' => [0x06, 0x08, 0x08, 0x1C, 0x08, 0x08, 0x08],
        'g' => [0x00, 0x00, 0x0D, 0x13, 0x0F, 0x01, 0x0E],
        'h' => [0x10, 0x10, 0x12, 0x1E, 0x11, 0x11, 0x11],
        'i' => [0x04, 0x00, 0x0C, 0x04, 0x04, 0x04, 0x0E],
        'j' => [0x02, 0x00, 0x06, 0x02, 0x02, 0x12, 0x0C],
        'k' => [0x10, 0x10, 0x11, 0x12, 0x1C, 0x12, 0x11],
        'l' => [0x0C, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'm' => [0x00, 0x00, 0x1A, 0x15, 0x15, 0x11, 0x11],
        'n' => [0x00, 0x00, 0x1E, 0x11, 0x11, 0x11, 0x11],
        'o' => [0x00, 0x00, 0x0E, 0x11, 0x11, 0x11, 0x0E],
        'p' => [0x00, 0x00, 0x1E, 0x11, 0x1E, 0x10, 0x10],
        'q' => [0x00, 0x00, 0x0D, 0x13, 0x0F, 0x01, 0x01],
        'r' => [0x00, 0x00, 0x16, 0x18, 0x10, 0x10, 0x10],
        's' => [0x00, 0x00, 0x0E, 0x10, 0x0E, 0x01, 0x0E],
        't' => [0x08, 0x08, 0x1C, 0x08, 0x08, 0x09, 0x06],
        'u' => [0x00, 0x00, 0x11, 0x11, 0x11, 0x13, 0x0D],
        'v' => [0x00, 0x00, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'w' => [0x00, 0x00, 0x11, 0x11, 0x15, 0x15, 0x0A],
        'x' => [0x00, 0x00, 0x11, 0x0A, 0x04, 0x0A, 0x11],
        'y' => [0x00, 0x00, 0x11, 0x11, 0x0F, 0x01, 0x0E],
        'z' => [0x00, 0x00, 0x1F, 0x02, 0x04, 0x08, 0x1F],
        ':' => [0x00, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x04],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x08],
        '=' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x1F, 0x00],
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '[' => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E],
        ']' => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fps_initially_zero() {
        let overlay = DebugOverlay::new();
        assert_eq!(overlay.fps, 0.0);
        assert_eq!(overlay.frame_time_ms, 0.0);
    }

    #[test]
    fn test_toggle() {
        let mut overlay = DebugOverlay::new();
        assert!(!overlay.is_visible());
        overlay.toggle();
        assert!(overlay.is_visible());
        overlay.toggle();
        assert!(!overlay.is_visible());
    }

    #[test]
    fn test_visible_default_false() {
        assert!(!DebugOverlay::new().is_visible());
    }

    #[test]
    fn test_fps_ema_converges() {
        let mut overlay = DebugOverlay::new();
        // Simulate 60 frames at ~60fps
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            overlay.update_fps(dt);
        }
        // EMA should be close to 60 after 60 frames
        assert!((overlay.fps - 60.0).abs() < 2.0, "fps={}", overlay.fps);
    }

    #[test]
    fn test_bitmap_for_space() {
        assert_eq!(DebugOverlay::bitmap_for(' '), [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_bitmap_for_digit() {
        // '1' should have middle column set
        let b = DebugOverlay::bitmap_for('1');
        assert_eq!(b[0], 0x04); // top row: ..X..
        assert_eq!(b[3], 0x04); // middle row: ..X..
    }
}
