//! In-game developer console — toggled by tilde key (`).
//! Provides command-line access to engine internals for debugging.

use std::collections::VecDeque;

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use super::debug::DebugOverlay;
use crate::core::{SceneData, SceneState, Camera};
use crate::game::Player;

/// 8x12 bitmap font dimensions (shared with debug.rs).
const CHAR_W: i32 = 8;
const CHAR_H: i32 = 12;
const CHAR_SPACING: i32 = 1;
const CELL_W: i32 = CHAR_W + CHAR_SPACING;
/// sprint-9: 3x scale for readability on mobile/desktop
const FONT_SCALE: i32 = 3;

/// Maximum number of output lines shown above the input.
const MAX_OUTPUT_LINES: usize = 20;

/// Maximum command history entries.
const MAX_HISTORY: usize = 50;

/// Height of the console panel in pixels.
const CONSOLE_H: i32 = 400;

/// Padding inside the console panel.
const PAD: i32 = 8;

/// Prompt character shown before input.
const PROMPT: char = '>';

pub struct Console {
    /// Whether the console is currently visible.
    pub visible: bool,
    /// Current input buffer (what the user is typing).
    pub input: String,
    /// Previously entered commands.
    pub history: VecDeque<String>,
    /// Output lines to display (recent at bottom).
    pub output: VecDeque<String>,
    /// Cursor blink timer (seconds).
    pub cursor_blink: f32,
    /// History navigation index (0 = not navigating, 1+ = position in history).
    history_index: usize,
}

impl Console {
    pub fn new() -> Self {
        Self {
            visible: false,
            input: String::new(),
            history: VecDeque::new(),
            output: VecDeque::new(),
            cursor_blink: 0.0,
            history_index: 0,
        }
    }

    /// Toggle console visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.input.clear();
            self.history_index = 0;
        }
    }

    /// Dismiss the console (same as toggle when visible).
    pub fn dismiss(&mut self) {
        self.visible = false;
        self.input.clear();
        self.history_index = 0;
    }

    /// Update cursor blink timer. Call each frame.
    pub fn tick(&mut self, dt: f32) {
        self.cursor_blink += dt;
        if self.cursor_blink >= 1.0 {
            self.cursor_blink -= 1.0;
        }
    }

    /// Handle a character key press while the console is open.
    pub fn handle_char(&mut self, c: char) {
        // Only accept printable ASCII
        if c.is_ascii_graphic() || c == ' ' {
            self.input.push(c);
            self.history_index = 0; // Reset history navigation on new input
        }
    }

    /// Handle a backspace key press while the console is open.
    pub fn handle_backspace(&mut self) {
        self.input.pop();
        self.history_index = 0;
    }

    /// Handle an enter/return key press — execute the current command.
    pub fn handle_enter(&mut self) {
        let cmd = self.input.trim();
        if !cmd.is_empty() {
            self.history.push_back(cmd.to_string());
            if self.history.len() > MAX_HISTORY {
                self.history.pop_front();
            }
        }
        self.input.clear();
        self.history_index = 0;
    }

    /// Handle up arrow — navigate to previous command in history.
    pub fn handle_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.history_index == 0 {
            // Save current input before navigating
            self.history_index = self.history.len();
        }
        if self.history_index > 1 {
            self.history_index -= 1;
        }
        // Show the historical command at history_index - 1
        self.input = self.history[self.history_index - 1].clone();
    }

    /// Handle down arrow — navigate to next (newer) command in history.
    pub fn handle_down(&mut self) {
        if self.history_index == 0 {
            return;
        }
        if self.history_index < self.history.len() {
            self.history_index += 1;
            self.input = self.history[self.history_index - 1].clone();
        } else {
            // Gone past the end — restore empty input
            self.history_index = 0;
            self.input.clear();
        }
    }

    /// Execute a command string and return output lines.
    pub fn execute(
        &mut self,
        input: &str,
        scene: &mut SceneData,
        player1: &mut Player,
        player2: &mut Player,
        camera: &mut Camera,
        god_mode: &mut bool,
        debug: &mut DebugOverlay,
    ) -> Vec<String> {
        let mut results = Vec::new();
        let cmd = input.trim();

        if cmd.is_empty() {
            return results;
        }

        let mut parts = cmd.split_whitespace();
        let command = match parts.next() {
            Some(c) => c.to_lowercase(),
            None => return results,
        };

        match command.as_str() {
            "help" => {
                results.push("Available commands:".to_string());
                results.push("  help              - List all commands".to_string());
                results.push("  cam <gx> <gy>     - Teleport camera to grid position".to_string());
                results.push("  p1 <gx> <gy>      - Teleport player 1 to grid position".to_string());
                results.push("  p2 <gx> <gy>      - Teleport player 2 to grid position".to_string());
                results.push("  scene <name>      - Jump to scene".to_string());
                results.push("  god [on|off]      - Toggle or set god mode".to_string());
                results.push("  state             - Print full game state summary".to_string());
                results.push("  fps               - Toggle FPS display".to_string());
                results.push("  screenshot        - Trigger a screenshot".to_string());
                results.push("  exit              - Close console".to_string());
            }
            "cam" => {
                let gx: Option<i32> = parts.next().and_then(|s| s.parse().ok());
                let gy: Option<i32> = parts.next().and_then(|s| s.parse().ok());
                match (gx, gy) {
                    (Some(x), Some(y)) => {
                        camera.center_on(x as f32, y as f32);
                        results.push(format!("Camera teleported to ({}, {})", x, y));
                    }
                    _ => {
                        results.push("Usage: cam <gx> <gy>".to_string());
                    }
                }
            }
            "p1" => {
                let gx: Option<i32> = parts.next().and_then(|s| s.parse().ok());
                let gy: Option<i32> = parts.next().and_then(|s| s.parse().ok());
                match (gx, gy) {
                    (Some(x), Some(y)) => {
                        player1.grid_x = x;
                        player1.grid_y = y;
                        results.push(format!("Player 1 teleported to ({}, {})", x, y));
                    }
                    _ => {
                        results.push("Usage: p1 <gx> <gy>".to_string());
                    }
                }
            }
            "p2" => {
                let gx: Option<i32> = parts.next().and_then(|s| s.parse().ok());
                let gy: Option<i32> = parts.next().and_then(|s| s.parse().ok());
                match (gx, gy) {
                    (Some(x), Some(y)) => {
                        player2.grid_x = x;
                        player2.grid_y = y;
                        results.push(format!("Player 2 teleported to ({}, {})", x, y));
                    }
                    _ => {
                        results.push("Usage: p2 <gx> <gy>".to_string());
                    }
                }
            }
            "scene" => {
                let name = match parts.next() {
                    Some(n) => n.to_lowercase(),
                    None => {
                        results.push("Usage: scene <name>".to_string());
                        results.push("  Valid scenes: menu, titlecard, playing, gameover, victory, paused".to_string());
                        return results;
                    }
                };
                match name.as_str() {
                    "menu" => {
                        scene.state = SceneState::Menu;
                        results.push("Scene: Menu".to_string());
                    }
                    "titlecard" => {
                        scene.state = SceneState::TitleCard;
                        scene.title_timer = 3.0;
                        results.push("Scene: TitleCard".to_string());
                    }
                    "playing" => {
                        scene.state = SceneState::Playing;
                        results.push("Scene: Playing".to_string());
                    }
                    "gameover" => {
                        scene.state = SceneState::GameOver;
                        scene.gameover_timer = 5.0;
                        results.push("Scene: GameOver".to_string());
                    }
                    "victory" => {
                        scene.state = SceneState::Victory;
                        scene.victory_timer = 10.0;
                        results.push("Scene: Victory".to_string());
                    }
                    "paused" => {
                        scene.state = SceneState::Paused;
                        results.push("Scene: Paused".to_string());
                    }
                    _ => {
                        results.push(format!("Unknown scene: {}", name));
                        results.push("  Valid scenes: menu, titlecard, playing, gameover, victory, paused".to_string());
                    }
                }
            }
            "god" => {
                let arg = parts.next();
                match arg {
                    Some("on") => {
                        *god_mode = true;
                        results.push("God mode: ON".to_string());
                    }
                    Some("off") => {
                        *god_mode = false;
                        results.push("God mode: OFF".to_string());
                    }
                    None => {
                        *god_mode = !*god_mode;
                        results.push(format!("God mode: {}", if *god_mode { "ON" } else { "OFF" }));
                    }
                    Some(a) => {
                        results.push(format!("Usage: god [on|off] (got: {})", a));
                    }
                }
            }
            "state" => {
                // Print full game state summary (immutable borrow)
                results.push("=== Game State ===".to_string());
                results.push(format!("Scene: {:?}", scene.state));
                results.push(format!("Time: {:.1}s", scene.time_elapsed));
                results.push(format!("Episode: {}", scene.save_data.last_episode.as_ref().unwrap_or(&"none".to_string())));
                results.push(format!("God mode: {}", *god_mode));
                results.push(format!("FPS visible: {}", debug.is_fps_visible()));
                results.push(format!("P1: ({}, {}) {:?}", player1.grid_x, player1.grid_y, player1.state));
                results.push(format!("P2: ({}, {}) {:?}", player2.grid_x, player2.grid_y, player2.state));
                results.push(format!("Camera: ({:.1}, {:.1})", camera.x, camera.y));
            }
            "fps" => {
                debug.toggle_fps();
                results.push(format!("FPS display: {}", if debug.is_fps_visible() { "ON" } else { "OFF" }));
            }
            "screenshot" => {
                // Return a special marker that lib.rs will interpret
                results.push("[SCREENSHOT]".to_string());
            }
            "exit" => {
                self.visible = false;
                results.push("Console closed".to_string());
            }
            _ => {
                results.push(format!("unknown command: {}", command));
                results.push("Type 'help' for available commands".to_string());
            }
        }

        // Append results to output history (max MAX_OUTPUT_LINES lines)
        for line in &results {
            self.output.push_back(line.clone());
        }
        while self.output.len() > MAX_OUTPUT_LINES {
            self.output.pop_front();
        }

        results
    }

    /// Render the console overlay on top of the game.
    pub fn render(&self, canvas: &mut Canvas<Window>) {
        if !self.visible {
            return;
        }

        let (screen_w, _screen_h) = canvas.output_size().unwrap_or((1280, 720));

        // Full-width dark background
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 200));
        let _ = canvas.fill_rect(Rect::new(0, 0, screen_w, CONSOLE_H as u32));

        // Top border line
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(80, 200, 100, 255));
        let _ = canvas.draw_rect(Rect::new(0, 0, screen_w, 1));

        let mut y = PAD;

        // Draw output history
        let visible_output_count = self.output.len().min(MAX_OUTPUT_LINES.saturating_sub(2));
        let output_start = self.output.len().saturating_sub(visible_output_count);

        for line in self.output.iter().skip(output_start) {
            DebugOverlay::draw_text(
                canvas,
                line,
                PAD,
                y,
                sdl2::pixels::Color::RGBA(180, 255, 180, 255),
            );
            y += CHAR_H * FONT_SCALE;
            if y + CHAR_H * FONT_SCALE > CONSOLE_H - PAD - CHAR_H * FONT_SCALE {
                break;
            }
        }

        // Draw input line at bottom
        y = CONSOLE_H - PAD - CHAR_H * FONT_SCALE;

        // Draw prompt character
        DebugOverlay::draw_text(
            canvas,
            &PROMPT.to_string(),
            PAD,
            y,
            sdl2::pixels::Color::RGBA(100, 255, 100, 255),
        );

        // Draw current input text
        let input_x = PAD + CELL_W * FONT_SCALE;
        DebugOverlay::draw_text(
            canvas,
            &self.input,
            input_x,
            y,
            sdl2::pixels::Color::RGBA(220, 255, 220, 255),
        );

        // Draw blinking cursor after input
        let cursor_x = input_x + (self.input.len() as i32 * CELL_W * FONT_SCALE);
        let show_cursor = self.cursor_blink < 0.5;
        if show_cursor {
            // Draw cursor as underscore using the bitmap font
            DebugOverlay::draw_char(
                canvas,
                '_',
                cursor_x,
                y + CHAR_H * FONT_SCALE - 2 * FONT_SCALE,
                sdl2::pixels::Color::RGBA(220, 255, 220, 255),
            );
        }
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}
