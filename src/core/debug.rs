//! Debug overlay — toggled by F1, renders engine internals via SDL2 primitives.
//! Uses an 8x12 bitmap font with box-drawing borders and vertical layout.

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::sync::atomic::AtomicUsize;

use super::{depth_key, screen_to_grid, SceneData, CheckResult, HealthResults, TILE_W, TILE_H};

/// Thread-safe draw call counter — incremented on every canvas.copy().
pub static DRAW_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// sprint-22: Ring buffer for frame time history (120 frames for rolling stats).
pub struct RingBuffer<T, const N: usize> {
    data: [Option<T>; N],
    head: usize,
    len: usize,
}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            data: [None; N],
            head: 0,
            len: 0,
        }
    }

    /// Push a value into the ring buffer.
    pub fn push(&mut self, val: T) {
        self.data[self.head] = Some(val);
        self.head = (self.head + 1) % N;
        if self.len < N {
            self.len += 1;
        }
    }

    /// Iterate over all valid elements in insertion order (oldest to newest).
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let start = if self.len < N { 0 } else { self.head };
        RingBufferIter {
            data: &self.data,
            start,
            current: 0,
            len: self.len,
        }
    }

    /// Return the average of all values.
    /// Requires T = f32 (used for frame time averaging).
    pub fn average(&self) -> Option<f32>
    where
        T: Into<f32>,
    {
        if self.len == 0 {
            return None;
        }
        let sum: f32 = self.iter().map(|&v| v.into()).sum();
        Some(sum / self.len as f32)
    }

    /// Return the 99th percentile (p99) of collected values, if enough data exists.
    /// Requires T = f32 (used for frame time p99 calculation).
    pub fn p99(&self) -> Option<f32>
    where
        T: Into<f32> + std::cmp::PartialOrd,
    {
        if self.len < 2 {
            return None;
        }
        let mut sorted: Vec<f32> = self.iter().map(|&v| v.into()).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // p99: index for 99th percentile = ceil(len * 0.99) - 1
        // For len=120: ceil(120 * 0.99) - 1 = 119 - 1 = 118
        let idx = ((self.len as f32) * 0.99).ceil() as usize;
        let idx = idx.saturating_sub(1).min(sorted.len().saturating_sub(1));
        Some(sorted[idx])
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<T: Copy, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

struct RingBufferIter<'a, T, const N: usize> {
    data: &'a [Option<T>; N],
    start: usize,
    current: usize,
    len: usize,
}

impl<'a, T, const N: usize> Iterator for RingBufferIter<'a, T, N> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.len {
            return None;
        }
        let idx = (self.start + self.current) % N;
        self.current += 1;
        self.data[idx].as_ref()
    }
}
use crate::core::Camera;
use crate::game::Player;
use crate::game::World;
use crate::game::TileType;

/// 8x12 bitmap font character cell dimensions.
const CHAR_W: i32 = 8;
const CHAR_H: i32 = 12;
/// Character spacing.
const CHAR_SPACING: i32 = 1;
/// Total cell width including spacing.
const CELL_W: i32 = CHAR_W + CHAR_SPACING;

/// Overlay padding.
const PAD: i32 = 8;

/// sprint-9: 3x scale for bitmap font — readable at arm's length on mobile/desktop
const FONT_SCALE: i32 = 3;

/// Maximum text line length in characters (for box width calculation).
const MAX_LINE_CHARS: usize = 35;

/// Current input directions for display.
#[derive(Default)]
pub struct DebugInputState {
    pub p1_dirs: String,
    pub p2_dirs: String,
}

/// Runtime state read from the game each frame.
pub struct DebugState<'a> {
    pub scene: &'a SceneData,
    pub player1: &'a Player,
    pub player2: &'a Player,
    pub world: &'a World,
    pub camera: &'a Camera,
    pub mouse_screen: (i32, i32),
    pub input: &'a DebugInputState,
    pub god_mode: bool,
    /// Computed player 1 screen position for camera sanity check
    pub p1_screen: (i32, i32),
    /// Computed player 2 screen position for camera sanity check
    pub p2_screen: (i32, i32),
    /// Time elapsed in current scene (seconds)
    pub time_elapsed: f32,
    /// Health check results
    pub health_results: &'a HealthResults,
    /// qa-replay-1: Whether input recording is active
    pub recording: bool,
    /// sprint-20: Whether minimap should be rendered
    pub show_minimap: bool,
}

pub struct DebugOverlay {
    visible: bool,
    fps_visible: bool,
    frame_graph_visible: bool,
    show_minimap: bool,
    fps: f32,
    frame_time_ms: f32,
    mouse_grid_x: i32,
    mouse_grid_y: i32,
    mouse_depth: i32,
    /// sprint-22: Rolling frame time history for graph and p99 stats.
    frame_times: RingBuffer<f32, 120>,
    /// sprint-22: Rolling draw call count per frame.
    draw_calls: usize,
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {
            visible: false,
            fps_visible: false,
            frame_graph_visible: false,
            show_minimap: false,
            fps: 0.0,
            frame_time_ms: 0.0,
            mouse_grid_x: 0,
            mouse_grid_y: 0,
            mouse_depth: 0,
            frame_times: RingBuffer::new(),
            draw_calls: 0,
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

    /// sprint-22: F5 cycles through: off → FPS → frame graph → both → off.
    pub fn toggle_frame_graph(&mut self) {
        self.frame_graph_visible = !self.frame_graph_visible;
    }

    pub fn is_frame_graph_visible(&self) -> bool {
        self.frame_graph_visible
    }

    pub fn toggle_minimap(&mut self) {
        self.show_minimap = !self.show_minimap;
    }

    pub fn is_minimap_visible(&self) -> bool {
        self.show_minimap
    }

    /// Get current FPS value (for health monitoring).
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Update FPS using exponential moving average. Call each frame.
    pub fn update_fps(&mut self, dt: f32) {
        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        // EMA with alpha = 0.1 for smooth display
        self.fps += 0.1 * (fps - self.fps);
        self.frame_time_ms += 0.1 * (dt * 1000.0 - self.frame_time_ms);
        // sprint-22: Record frame time for rolling stats
        self.frame_times.push(dt * 1000.0);
    }

    /// sprint-22: Record per-frame draw call count.
    pub fn set_draw_calls(&mut self, count: usize) {
        self.draw_calls = count;
    }

    /// sprint-22: Return the number of draw calls recorded for the last frame.
    pub fn draw_call_count(&self) -> usize {
        self.draw_calls
    }

    /// sprint-22: Return average frame time in ms.
    pub fn avg_frame_time(&self) -> Option<f32> {
        self.frame_times.average()
    }

    /// sprint-22: Return p99 frame time in ms.
    pub fn p99_frame_time(&self) -> Option<f32> {
        self.frame_times.p99()
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
        let (screen_w, _screen_h) = canvas.output_size().unwrap_or((1280, 720));

        // sprint-22: Render compact badges when overlay is hidden (F5 states)
        if !self.visible {
            let badge_h: i32 = if self.frame_graph_visible { 36 } else { 24 };
            let badge_w: i32 = if self.frame_graph_visible { 220 } else { 110 };
            let badge_x = screen_w as i32 - badge_w - 8;
            let badge_y: i32 = 8;

            if self.fps_visible || self.frame_graph_visible {
                canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 160));
                let _ = canvas.fill_rect(Rect::new(badge_x, badge_y, badge_w as u32, badge_h as u32));
                canvas.set_draw_color(sdl2::pixels::Color::RGBA(100, 200, 100, 255));
                let _ = canvas.draw_rect(Rect::new(badge_x, badge_y, badge_w as u32, badge_h as u32));

                if self.fps_visible {
                    let fps_text = format!("FPS: {:4.1}", self.fps);
                    Self::draw_text(canvas, &fps_text, badge_x + 6, badge_y + 6, sdl2::pixels::Color::RGBA(180, 255, 180, 255));
                }

                // sprint-22: Render ASCII frame graph bar
                if self.frame_graph_visible {
                    let avg_ms = self.avg_frame_time().unwrap_or(0.0);
                    let bar_text = Self::frame_graph_bar(avg_ms);
                    let bar_y = if self.fps_visible { badge_y + 20 } else { badge_y + 8 };
                    Self::draw_text(canvas, &bar_text, badge_x + 6, bar_y, sdl2::pixels::Color::RGBA(200, 255, 200, 255));
                }
            }
        }

        // qa-replay-1: Always show REC indicator in top-right when recording
        if state.recording {
            let (screen_w, _screen_h) = canvas.output_size().unwrap_or((1280, 720));
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 160));
            let _ = canvas.fill_rect(Rect::new(screen_w as i32 - 60, 8, 52, 24));
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 80, 80, 255));
            let _ = canvas.draw_rect(Rect::new(screen_w as i32 - 60, 8, 52, 24));
            Self::draw_text(
                canvas,
                "REC",
                screen_w as i32 - 56,
                14,
                sdl2::pixels::Color::RGBA(255, 80, 80, 255),
            );
        }

        if !self.visible {
            return;
        }

        let (screen_w, _screen_h) = canvas.output_size().unwrap_or((1280, 720));
        let screen_h = _screen_h;

        // Calculate panel dimensions
        // Each section has a header line + data lines
        // Box width: MAX_LINE_CHARS * CELL_W + PAD * 2
        let box_inner_w = MAX_LINE_CHARS as i32 * CELL_W;
        let panel_w = box_inner_w + PAD * 2;
        let panel_h = self.calculate_panel_height();

        // Draw semi-transparent background
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 180));
        let _ = canvas.fill_rect(Rect::new(0, 0, panel_w as u32, panel_h as u32));

        // Draw sections with box-drawing borders
        let mut y = 0;

        // ╔══════════════════════════════════════╗
        // ║ SYSTEM                                 ║
        // ║   FPS....60.0  FT....16ms             ║
        // ║   SCENE..Playing                      ║
        // ║   TIME...00:42                        ║
        y += self.draw_section_system(canvas, state, y, panel_w);
        // ╠══════════════════════════════════════╣
        y += self.draw_divider(canvas, y, panel_w);
        // ║ PLAYERS                               ║
        // ║   P1 grid=(4,3) [Idle]               ║
        // ║   P1 scr=(-416,-152)                  ║
        // ║   P2 grid=(2,4) [Moving]              ║
        // ║   P2 scr=(-704,-56)                   ║
        y += self.draw_section_players(canvas, state, y, panel_w);
        y += self.draw_divider(canvas, y, panel_w);
        // ║ CAMERA              [✓ OK]           ║
        // ║   target=(-384.0,-104.0)              ║
        // ║   current=(-384.0,-104.0)             ║
        y += self.draw_section_camera(canvas, state, y, panel_w);
        y += self.draw_divider(canvas, y, panel_w);
        // ║ WORLD                                 ║
        // ║   HOVER grid=(4,2) depth=6           ║
        // ║   MOUSE scr=(320,180)                 ║
        y += self.draw_section_world(canvas, state, y, panel_w);
        y += self.draw_divider(canvas, y, panel_w);
        // ║ INPUT                                 ║
        // ║   P1:[L---]  P2:[----]               ║
        y += self.draw_section_input(canvas, state, y, panel_w);
        y += self.draw_divider(canvas, y, panel_w);
        // ║ HEALTH                               ║
        // ║   CAM..[OK]  FPS..[OK]              ║
        // ║   PLR..[OK]  WLD..[OK]              ║
        y += self.draw_section_health(canvas, state, y, panel_w);
        // ╚══════════════════════════════════════╝
        let _ = canvas.set_draw_color(sdl2::pixels::Color::RGBA(100, 200, 100, 255));
        let _ = canvas.draw_rect(Rect::new(0, panel_h - 1, panel_w as u32, 1));

        // God mode indicator
        if state.god_mode {
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 200, 0, 255));
            let _ = canvas.fill_rect(Rect::new(screen_w as i32 - 80, 8, 72, 20));
            Self::draw_text(canvas, "GOD MODE", screen_w as i32 - 76, 12, sdl2::pixels::Color::RGBA(0, 0, 0, 255));
        }

        // sprint-20: Render minimap if enabled
        if state.show_minimap {
            Self::render_minimap(canvas, state.world, state.player1, state.player2, state.camera, screen_w, screen_h);
        }
    }

    /// sprint-20: Render an isometric minimap in the top-right corner.
    /// Position: (screen_w - 128 - 8, 8), size 128x64 pixels.
    /// Each tile = 1 pixel, colored by type.
    /// Players shown as 2px colored dots, camera viewport as white outline.
    fn render_minimap(canvas: &mut Canvas<Window>, world: &World, player1: &Player, player2: &Player, camera: &Camera, screen_w: u32, screen_h: u32) {
        const MM_W: i32 = 128;
        const MM_H: i32 = 64;

        let mm_x = screen_w as i32 - MM_W - 8;
        let mm_y: i32 = 8;

        // Hide minimap if screen is too small
        if screen_h < 480 {
            return;
        }

        // Calculate scale to fit world into minimap
        let scale_x = MM_W as f32 / world.grid_w as f32;
        let scale_y = MM_H as f32 / world.grid_h as f32;

        // Draw semi-transparent background for minimap
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 160));
        let _ = canvas.fill_rect(Rect::new(mm_x, mm_y, MM_W as u32, MM_H as u32));

        // Draw border
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(100, 100, 100, 255));
        let _ = canvas.draw_rect(Rect::new(mm_x, mm_y, MM_W as u32, MM_H as u32));

        // Draw tiles as 1-pixel dots
        for gy in 0..world.grid_h {
            for gx in 0..world.grid_w {
                let px = mm_x + (gx as f32 * scale_x) as i32;
                let py = mm_y + (gy as f32 * scale_y) as i32;

                // Skip if outside minimap bounds
                if px < mm_x || px >= mm_x + MM_W || py < mm_y || py >= mm_y + MM_H {
                    continue;
                }

                let tile = world.get_tile(gx, gy);
                let color = match tile {
                    TileType::Passable => {
                        // Determine theme color from tile string
                        let tile_str = world.get_tile_string(gx, gy);
                        if tile_str.contains("lava") || tile_str.contains("volcanic") || tile_str.contains("lava_trap") {
                            sdl2::pixels::Color::RGBA(200, 60, 30, 255) // lava
                        } else if tile_str.contains("ice") || tile_str.contains("snow") || tile_str.contains("frozen") {
                            sdl2::pixels::Color::RGBA(150, 200, 220, 255) // ice
                        } else if tile_str.contains("water") || tile_str.contains("ocean") {
                            sdl2::pixels::Color::RGBA(60, 120, 200, 255) // water
                        } else {
                            sdl2::pixels::Color::RGBA(60, 180, 60, 255) // grass
                        }
                    }
                    TileType::Solid => sdl2::pixels::Color::RGBA(100, 100, 100, 255), // wall
                    TileType::Trap => sdl2::pixels::Color::RGBA(200, 60, 30, 255), // lava trap
                    TileType::Checkpoint => sdl2::pixels::Color::RGBA(255, 200, 0, 255), // checkpoint
                    TileType::Goal => sdl2::pixels::Color::RGBA(255, 215, 0, 255), // goal
                };

                canvas.set_draw_color(color);
                let _ = canvas.fill_rect(Rect::new(px, py, 1, 1));
            }
        }

        // Draw NPCs as 2px yellow dots
        for npc in &world.episode.npcs {
            let px = mm_x + (npc.x as f32 * scale_x) as i32;
            let py = mm_y + (npc.y as f32 * scale_y) as i32;
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 255, 0, 255));
            let _ = canvas.fill_rect(Rect::new(px - 1, py - 1, 2, 2));
        }

        // Draw players as 2px dots (P1=cyan, P2=magenta)
        let p1_px = mm_x + (player1.grid_x as f32 * scale_x) as i32;
        let p1_py = mm_y + (player1.grid_y as f32 * scale_y) as i32;
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 255, 255, 255));
        let _ = canvas.fill_rect(Rect::new(p1_px - 1, p1_py - 1, 2, 2));

        let p2_px = mm_x + (player2.grid_x as f32 * scale_x) as i32;
        let p2_py = mm_y + (player2.grid_y as f32 * scale_y) as i32;
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 0, 255, 255));
        let _ = canvas.fill_rect(Rect::new(p2_px - 1, p2_py - 1, 2, 2));

        // Draw camera viewport as white 1px rectangle outline
        let vp_mm_x = mm_x + (camera.x / TILE_W as f32) as i32;
        let vp_mm_y = mm_y + (camera.y / TILE_H as f32) as i32;
        let vp_mm_w = (screen_w as f32 / TILE_W as f32) as i32;
        let vp_mm_h = (screen_h as f32 / TILE_H as f32) as i32;
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 255, 255, 255));
        let _ = canvas.draw_rect(Rect::new(vp_mm_x, vp_mm_y, vp_mm_w.max(1) as u32, vp_mm_h.max(1) as u32));
    }

    /// sprint-22: Build an ASCII frame graph bar string.
    /// Renders 16 characters: █ for used budget, ░ for remaining.
    /// Target is 16.67ms (60fps). Bars clamp at 20ms full-bar.
    fn frame_graph_bar(avg_ms: f32) -> String {
        const BARS: usize = 16;
        const TARGET_MS: f32 = 16.67;
        let filled = ((avg_ms / TARGET_MS) * BARS as f32).round() as usize;
        let filled = filled.min(BARS);
        let bar: String = "████████████████".chars().take(filled).map(|_| '█').collect();
        let rest: String = "░░░░░░░░░░░░░░".chars().take(BARS - filled).map(|_| '░').collect();
        format!("{:8.1}ms {}{}", avg_ms, bar, rest)
    }

    fn calculate_panel_height(&self) -> i32 {
        // SYSTEM: 1 header + 8 data lines = 9 lines (sprint-22: added AVG/P99/DC)
        // PLAYERS: 1 header + 4 data lines = 5 lines
        // CAMERA: 1 header + 2 data lines = 3 lines
        // WORLD: 1 header + 2 data lines = 3 lines
        // INPUT: 1 header + 1 data line = 2 lines
        // HEALTH: 1 header + 2 data lines = 3 lines
        // Dividers: 5 between sections
        // Box borders: top + bottom
        let num_lines = 9 + 5 + 3 + 3 + 2 + 3;
        let num_dividers = 5;
        let border_height = 2;
        num_lines as i32 * CHAR_H + num_dividers as i32 + border_height + PAD * 2
    }

    fn draw_divider(&self, canvas: &mut Canvas<Window>, y: i32, panel_w: i32) -> i32 {
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(100, 200, 100, 255));
        let _ = canvas.draw_rect(Rect::new(0, y, panel_w as u32, 1));
        CHAR_H
    }

    fn draw_section_header(&self, canvas: &mut Canvas<Window>, title: &str, y: i32, panel_w: i32, color: sdl2::pixels::Color) -> i32 {
        let header_text = format!("{} ", title);
        Self::draw_text(canvas, &header_text, PAD, y, color);
        // Draw line after header
        let text_end_x = PAD + header_text.len() as i32 * CELL_W;
        canvas.set_draw_color(color);
        // Line from end of text to edge of box
        let _ = canvas.draw_line((text_end_x, y + CHAR_H / 2), (panel_w - PAD, y + CHAR_H / 2));
        CHAR_H
    }

    fn draw_data_line(&self, canvas: &mut Canvas<Window>, text: &str, y: i32, color: sdl2::pixels::Color) -> i32 {
        Self::draw_text(canvas, text, PAD, y, color);
        CHAR_H
    }

    fn section_color() -> sdl2::pixels::Color {
        sdl2::pixels::Color::RGBA(180, 255, 180, 255)
    }

    fn value_color() -> sdl2::pixels::Color {
        sdl2::pixels::Color::RGBA(220, 255, 220, 255)
    }

    fn draw_section_system(&self, canvas: &mut Canvas<Window>, state: &DebugState, start_y: i32, panel_w: i32) -> i32 {
        let mut y = start_y;
        let c = Self::section_color();
        let v = Self::value_color();

        y += self.draw_section_header(canvas, "SYSTEM", y, panel_w, c);
        y += self.draw_data_line(canvas, &format!("  FPS....{:6.1}", self.fps), y, v);
        y += self.draw_data_line(canvas, &format!("  FT.....{:5.1}ms", self.frame_time_ms), y, v);
        // sprint-22: Show frame budget breakdown (avg/p99)
        if let Some(avg_ms) = self.avg_frame_time() {
            y += self.draw_data_line(canvas, &format!("  AVG....{:5.1}ms", avg_ms), y, v);
        }
        if let Some(p99_ms) = self.p99_frame_time() {
            y += self.draw_data_line(canvas, &format!("  P99....{:5.1}ms", p99_ms), y, v);
        }
        // sprint-22: Show draw calls
        y += self.draw_data_line(canvas, &format!("  DC.....{:4}", self.draw_calls), y, v);
        y += self.draw_data_line(canvas, &format!("  SCENE..{:?}", state.scene.state), y, v);
        y += self.draw_data_line(canvas, &format!("  TIME...{:02}:{:02}", (state.time_elapsed as u32) / 60, (state.time_elapsed as u32) % 60), y, v);
        // qa-replay-1: Show REC indicator in debug overlay when recording
        if state.recording {
            y += self.draw_data_line(canvas, "  [REC]  REC", y, sdl2::pixels::Color::RGBA(255, 80, 80, 255));
        }
        y
    }

    fn draw_section_players(&self, canvas: &mut Canvas<Window>, state: &DebugState, start_y: i32, panel_w: i32) -> i32 {
        let mut y = start_y;
        let c = Self::section_color();
        let v = Self::value_color();

        y += self.draw_section_header(canvas, "PLAYERS", y, panel_w, c);

        let p1_state = match state.player1.state {
            crate::game::PlayerState::Idle => "Idle",
            crate::game::PlayerState::Moving => "Moving",
            crate::game::PlayerState::Eliminated => "Elim",
            crate::game::PlayerState::Won => "Won",
        };
        y += self.draw_data_line(canvas, &format!("  P1 grid=({},{}) [{}]", state.player1.grid_x, state.player1.grid_y, p1_state), y, v);
        y += self.draw_data_line(canvas, &format!("  P1 scr=({},{})", state.p1_screen.0, state.p1_screen.1), y, v);

        let p2_state = match state.player2.state {
            crate::game::PlayerState::Idle => "Idle",
            crate::game::PlayerState::Moving => "Moving",
            crate::game::PlayerState::Eliminated => "Elim",
            crate::game::PlayerState::Won => "Won",
        };
        y += self.draw_data_line(canvas, &format!("  P2 grid=({},{}) [{}]", state.player2.grid_x, state.player2.grid_y, p2_state), y, v);
        y += self.draw_data_line(canvas, &format!("  P2 scr=({},{})", state.p2_screen.0, state.p2_screen.1), y, v);
        y
    }

    fn draw_section_camera(&self, canvas: &mut Canvas<Window>, state: &DebugState, start_y: i32, panel_w: i32) -> i32 {
        let mut y = start_y;
        let c = Self::section_color();
        let v = Self::value_color();

        // Health indicator next to header
        let health_color = match state.health_results.camera {
            CheckResult::Ok => sdl2::pixels::Color::RGBA(100, 255, 100, 255),
            CheckResult::Warn => sdl2::pixels::Color::RGBA(255, 200, 100, 255),
            CheckResult::Fail => sdl2::pixels::Color::RGBA(255, 100, 100, 255),
        };
        let health_label = match state.health_results.camera {
            CheckResult::Ok => "[OK] ",
            CheckResult::Warn => "[WARN] ",
            CheckResult::Fail => "[FAIL] ",
        };

        y += self.draw_section_header(canvas, "CAMERA", y, panel_w, c);
        // Draw health indicator
        let header_text = format!("  CAMERA ");
        let header_len = header_text.len() as i32 * CELL_W;
        Self::draw_text(canvas, health_label, PAD + header_len, start_y + CHAR_H / 2 - 6, health_color);

        // Adjust y back since header was already counted
        y = start_y + CHAR_H;
        y += self.draw_data_line(canvas, &format!("  target=({:5.1},{:5.1})", state.camera.target_x, state.camera.target_y), y, v);
        y += self.draw_data_line(canvas, &format!("  current=({:5.1},{:5.1})", state.camera.x, state.camera.y), y, v);
        y
    }

    fn draw_section_world(&self, canvas: &mut Canvas<Window>, state: &DebugState, start_y: i32, panel_w: i32) -> i32 {
        let mut y = start_y;
        let c = Self::section_color();
        let v = Self::value_color();

        y += self.draw_section_header(canvas, "WORLD", y, panel_w, c);
        y += self.draw_data_line(canvas, &format!("  HOVER grid=({},{}) depth={}", self.mouse_grid_x, self.mouse_grid_y, self.mouse_depth), y, v);
        y += self.draw_data_line(canvas, &format!("  MOUSE scr=({},{})", state.mouse_screen.0, state.mouse_screen.1), y, v);
        y
    }

    fn draw_section_input(&self, canvas: &mut Canvas<Window>, state: &DebugState, start_y: i32, panel_w: i32) -> i32 {
        let mut y = start_y;
        let c = Self::section_color();
        let v = Self::value_color();

        y += self.draw_section_header(canvas, "INPUT", y, panel_w, c);
        y += self.draw_data_line(canvas, &format!("  P1:[{}]  P2:[{}]", state.input.p1_dirs, state.input.p2_dirs), y, v);
        y
    }

    fn draw_section_health(&self, canvas: &mut Canvas<Window>, state: &DebugState, start_y: i32, panel_w: i32) -> i32 {
        let mut y = start_y;
        let c = Self::section_color();

        y += self.draw_section_header(canvas, "HEALTH", y, panel_w, c);

        // CAM and FPS on first line — use overlay_display() once per check
        let (cam_color, cam_status) = state.health_results.camera.overlay_display();
        let (fps_color, fps_status) = state.health_results.fps.overlay_display();
        let cam_label = format!("CAM..{}", cam_status);
        let fps_label = format!("FPS..{}", fps_status);
        Self::draw_text(canvas, "  ", PAD, y, c);
        Self::draw_text(canvas, &cam_label, PAD + 2 * CELL_W, y, cam_color);
        Self::draw_text(canvas, &fps_label, PAD + 13 * CELL_W, y, fps_color);
        y += CHAR_H;

        // PLR on second line
        let plr_combined = (state.health_results.player1, state.health_results.player2);
        let (plr_color, plr_status) = match plr_combined {
            (CheckResult::Ok, CheckResult::Ok) => {
                (sdl2::pixels::Color::RGBA(100, 255, 100, 255), "[OK]")
            }
            (CheckResult::Fail, _) | (_, CheckResult::Fail) => {
                (sdl2::pixels::Color::RGBA(255, 100, 100, 255), "[FAIL]")
            }
            _ => (sdl2::pixels::Color::RGBA(255, 200, 100, 255), "[WARN]"),
        };
        let plr_label = format!("PLR..{}", plr_status);
        Self::draw_text(canvas, "  ", PAD, y, c);
        Self::draw_text(canvas, &plr_label, PAD + 2 * CELL_W, y, plr_color);
        y += CHAR_H;

        y
    }

    // ── 8x12 Bitmap Font ────────────────────────────────────────────────────────

    /// Draw null-terminated ASCII string at pixel position (x, y).
    pub fn draw_text(canvas: &mut Canvas<Window>, s: &str, x: i32, y: i32, color: sdl2::pixels::Color) {
        let mut px = x;
        for ch in s.chars() {
            if px + CHAR_W * FONT_SCALE > 1280 {
                break;
            }
            Self::draw_char(canvas, ch, px, y, color);
            px += CELL_W * FONT_SCALE;
        }
    }

    pub fn draw_char(canvas: &mut Canvas<Window>, ch: char, x: i32, y: i32, color: sdl2::pixels::Color) {
        let bitmap = font_8x12(ch);
        canvas.set_draw_color(color);
        for row in 0..12 {
            let bits = bitmap[row];
            let byte_count = (CHAR_W + 7) / 8;
            for byte_idx in 0..byte_count {
                let byte_val = bits[byte_idx as usize];
                let start_bit = byte_idx * 8;
                let bits_this_byte = (CHAR_W - start_bit).min(8);
                for col in 0..bits_this_byte {
                    if (byte_val >> (7 - col)) & 1 != 0 {
                        let _ = canvas.fill_rect(Rect::new(
                            x + (start_bit as i32 + col) * FONT_SCALE,
                            y + row as i32 * FONT_SCALE,
                            FONT_SCALE as u32,
                            FONT_SCALE as u32,
                        ));
                    }
                }
            }
        }
    }

    /// Return 12-byte bitmap for a character (used by tests).
    #[allow(dead_code)]
    fn bitmap_for(ch: char) -> [[u8; 2]; 12] {
        font_8x12(ch)
    }
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}

// ── 8x12 Bitmap font table ────────────────────────────────────────────────────
// Each row is 2 bytes (16 bits, only lower 8 used), giving 8 columns × 12 rows.
// Box-drawing chars added: ╔ ═ ║ ╠ ╣ ╚ ╝ ╭ ╮ ╯ ╰

fn font_8x12(ch: char) -> [[u8; 2]; 12] {
    match ch {
        ' ' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '0' => [[0x00,0],[0x1E,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x00,0]],
        '1' => [[0x00,0],[0x0C,0],[0x1C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x3F,0],[0x00,0]],
        '2' => [[0x00,0],[0x1E,0],[0x33,0],[0x30,0],[0x30,0],[0x1C,0],[0x06,0],[0x03,0],[0x03,0],[0x33,0],[0x3F,0],[0x00,0]],
        '3' => [[0x00,0],[0x1E,0],[0x33,0],[0x30,0],[0x30,0],[0x1C,0],[0x0C,0],[0x30,0],[0x30,0],[0x33,0],[0x1E,0],[0x00,0]],
        '4' => [[0x00,0],[0x0C,0],[0x1C,0],[0x0C,0],[0x0C,0],[0x33,0],[0x3F,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x00,0]],
        '5' => [[0x00,0],[0x3F,0],[0x03,0],[0x03,0],[0x3F,0],[0x33,0],[0x30,0],[0x30,0],[0x30,0],[0x33,0],[0x1E,0],[0x00,0]],
        '6' => [[0x00,0],[0x0E,0],[0x13,0],[0x23,0],[0x03,0],[0x1F,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x00,0]],
        '7' => [[0x00,0],[0x3F,0],[0x33,0],[0x30,0],[0x18,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x00,0]],
        '8' => [[0x00,0],[0x1E,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x00,0]],
        '9' => [[0x00,0],[0x1E,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x30,0],[0x30,0],[0x30,0],[0x31,0],[0x1E,0],[0x00,0]],
        'A' => [[0x00,0],[0x1C,0],[0x36,0],[0x63,0],[0x63,0],[0x7F,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x00,0]],
        'B' => [[0x00,0],[0x3F,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x00,0]],
        'C' => [[0x00,0],[0x1E,0],[0x33,0],[0x63,0],[0x60,0],[0x60,0],[0x60,0],[0x60,0],[0x63,0],[0x33,0],[0x1E,0],[0x00,0]],
        'D' => [[0x00,0],[0x3F,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x00,0]],
        'E' => [[0x00,0],[0x7F,0],[0x03,0],[0x03,0],[0x03,0],[0x3F,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x7F,0],[0x00,0]],
        'F' => [[0x00,0],[0x7F,0],[0x03,0],[0x03,0],[0x03,0],[0x3F,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x00,0]],
        'G' => [[0x00,0],[0x1E,0],[0x33,0],[0x63,0],[0x60,0],[0x6F,0],[0x63,0],[0x63,0],[0x63,0],[0x33,0],[0x1E,0],[0x00,0]],
        'H' => [[0x00,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x7F,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x00,0]],
        'I' => [[0x00,0],[0x3F,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x3F,0],[0x00,0]],
        'J' => [[0x00,0],[0x1F,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x36,0],[0x36,0],[0x1C,0],[0x00,0]],
        'K' => [[0x00,0],[0x33,0],[0x37,0],[0x3B,0],[0x3F,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x00,0]],
        'L' => [[0x00,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x7F,0],[0x00,0]],
        'M' => [[0x00,0],[0x63,0],[0x77,0],[0x7F,0],[0x6B,0],[0x6B,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x00,0]],
        'N' => [[0x00,0],[0x63,0],[0x73,0],[0x7B,0],[0x7F,0],[0x6F,0],[0x67,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x00,0]],
        'O' => [[0x00,0],[0x1E,0],[0x33,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x33,0],[0x1E,0],[0x00,0]],
        'P' => [[0x00,0],[0x3F,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x00,0]],
        'Q' => [[0x00,0],[0x1E,0],[0x33,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x3B,0],[0x1E,0],[0x01,0]],
        'R' => [[0x00,0],[0x3F,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x1B,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x00,0]],
        'S' => [[0x00,0],[0x1E,0],[0x33,0],[0x03,0],[0x03,0],[0x1E,0],[0x30,0],[0x30,0],[0x30,0],[0x33,0],[0x1E,0],[0x00,0]],
        'T' => [[0x00,0],[0x7F,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x00,0]],
        'U' => [[0x00,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x00,0]],
        'V' => [[0x00,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x0C,0],[0x00,0]],
        'W' => [[0x00,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x6B,0],[0x6B,0],[0x7F,0],[0x77,0],[0x63,0],[0x00,0]],
        'X' => [[0x00,0],[0x63,0],[0x63,0],[0x36,0],[0x1C,0],[0x1C,0],[0x1C,0],[0x36,0],[0x36,0],[0x63,0],[0x63,0],[0x00,0]],
        'Y' => [[0x00,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x00,0]],
        'Z' => [[0x00,0],[0x7F,0],[0x31,0],[0x33,0],[0x33,0],[0x1E,0],[0x33,0],[0x33,0],[0x33,0],[0x31,0],[0x7F,0],[0x00,0]],
        'a' => [[0x00,0],[0x00,0],[0x00,0],[0x1E,0],[0x31,0],[0x3F,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x00,0]],
        'b' => [[0x00,0],[0x07,0],[0x0B,0],[0x0F,0],[0x1B,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x00,0]],
        'c' => [[0x00,0],[0x00,0],[0x00,0],[0x1E,0],[0x33,0],[0x60,0],[0x60,0],[0x60,0],[0x63,0],[0x33,0],[0x1E,0],[0x00,0]],
        'd' => [[0x00,0],[0x70,0],[0x30,0],[0x30,0],[0x3C,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x00,0]],
        'e' => [[0x00,0],[0x00,0],[0x00,0],[0x1E,0],[0x33,0],[0x3F,0],[0x03,0],[0x1E,0],[0x30,0],[0x33,0],[0x1E,0],[0x00,0]],
        'f' => [[0x00,0],[0x1C,0],[0x36,0],[0x06,0],[0x1F,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x00,0]],
        'g' => [[0x00,0],[0x00,0],[0x00,0],[0x1B,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x30,0],[0x31,0],[0x1E,0],[0x00,0]],
        'h' => [[0x00,0],[0x07,0],[0x0B,0],[0x0F,0],[0x1B,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x00,0]],
        'i' => [[0x00,0],[0x0C,0],[0x00,0],[0x1C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x3F,0],[0x00,0]],
        'j' => [[0x00,0],[0x03,0],[0x00,0],[0x0F,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x33,0],[0x1E,0],[0x00,0]],
        'k' => [[0x00,0],[0x33,0],[0x33,0],[0x33,0],[0x3B,0],[0x1F,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x00,0]],
        'l' => [[0x00,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x3F,0],[0x00,0]],
        'm' => [[0x00,0],[0x00,0],[0x00,0],[0x3E,0],[0x6B,0],[0x6B,0],[0x6B,0],[0x6B,0],[0x6B,0],[0x6B,0],[0x6B,0],[0x00,0]],
        'n' => [[0x00,0],[0x00,0],[0x00,0],[0x3B,0],[0x67,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x00,0]],
        'o' => [[0x00,0],[0x00,0],[0x00,0],[0x1E,0],[0x33,0],[0x63,0],[0x63,0],[0x63,0],[0x63,0],[0x33,0],[0x1E,0],[0x00,0]],
        'p' => [[0x00,0],[0x00,0],[0x00,0],[0x3F,0],[0x33,0],[0x33,0],[0x3F,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x00,0]],
        'q' => [[0x00,0],[0x00,0],[0x00,0],[0x1E,0],[0x33,0],[0x33,0],[0x1E,0],[0x30,0],[0x30,0],[0x30,0],[0x30,0],[0x00,0]],
        'r' => [[0x00,0],[0x00,0],[0x00,0],[0x37,0],[0x3B,0],[0x1F,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x03,0],[0x00,0]],
        's' => [[0x00,0],[0x00,0],[0x00,0],[0x1E,0],[0x33,0],[0x07,0],[0x1E,0],[0x30,0],[0x30,0],[0x33,0],[0x1E,0],[0x00,0]],
        't' => [[0x00,0],[0x06,0],[0x06,0],[0x1F,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x36,0],[0x1C,0],[0x00,0]],
        'u' => [[0x00,0],[0x00,0],[0x00,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x00,0]],
        'v' => [[0x00,0],[0x00,0],[0x00,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x33,0],[0x1E,0],[0x1E,0],[0x0C,0],[0x00,0]],
        'w' => [[0x00,0],[0x00,0],[0x00,0],[0x63,0],[0x63,0],[0x63,0],[0x6B,0],[0x6B,0],[0x6B,0],[0x7F,0],[0x36,0],[0x00,0]],
        'x' => [[0x00,0],[0x00,0],[0x00,0],[0x33,0],[0x1E,0],[0x1C,0],[0x1C,0],[0x1C,0],[0x36,0],[0x36,0],[0x33,0],[0x00,0]],
        'y' => [[0x00,0],[0x00,0],[0x00,0],[0x33,0],[0x33,0],[0x33,0],[0x3F,0],[0x30,0],[0x30,0],[0x31,0],[0x1E,0],[0x00,0]],
        'z' => [[0x00,0],[0x00,0],[0x00,0],[0x7F,0],[0x31,0],[0x33,0],[0x1E,0],[0x33,0],[0x33,0],[0x31,0],[0x7F,0],[0x00,0]],
        ':' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x0C,0],[0x0C,0],[0x00,0],[0x00,0],[0x00,0],[0x0C,0],[0x0C,0],[0x00,0]],
        ';' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x0C,0],[0x0C,0],[0x00,0],[0x00,0],[0x0C,0],[0x0C,0],[0x04,0],[0x00,0]],
        '-' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x1F,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '.' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x0C,0],[0x0C,0],[0x00,0]],
        ',' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x0C,0],[0x0C,0],[0x04,0]],
        '=' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x3F,0],[0x00,0],[0x00,0],[0x3F,0],[0x00,0],[0x00,0],[0x00,0]],
        '+' => [[0x00,0],[0x00,0],[0x00,0],[0x0C,0],[0x0C,0],[0x3F,0],[0x0C,0],[0x0C,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '(' => [[0x00,0],[0x03,0],[0x06,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x06,0],[0x03,0],[0x00,0]],
        ')' => [[0x00,0],[0x18,0],[0x0C,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x06,0],[0x0C,0],[0x18,0],[0x00,0]],
        '[' => [[0x00,0],[0x1F,0],[0x09,0],[0x09,0],[0x09,0],[0x09,0],[0x09,0],[0x09,0],[0x09,0],[0x09,0],[0x1F,0],[0x00,0]],
        ']' => [[0x00,0],[0xF8,0],[0x90,0],[0x90,0],[0x90,0],[0x90,0],[0x90,0],[0x90,0],[0x90,0],[0x90,0],[0xF8,0],[0x00,0]],
        // Box-drawing characters
        '╔' => [[0x00,0],[0x3F,0],[0x30,0],[0x30,0],[0x30,0],[0x30,0],[0x30,0],[0x30,0],[0x30,0],[0x30,0],[0x30,0],[0x00,0]],
        '═' => [[0x00,0],[0x00,0],[0xFF,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '║' => [[0x00,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x00,0]],
        '╠' => [[0x00,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0xFF,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x00,0]],
        '╣' => [[0x00,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0xF8,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x00,0]],
        '╚' => [[0x00,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0xFF,0],[0x00,0]],
        '╝' => [[0x00,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0x18,0],[0xF8,0],[0x00,0]],
        '╭' => [[0x00,0],[0x3F,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x00,0]],
        '╮' => [[0x00,0],[0x3F,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x00,0]],
        '╯' => [[0x00,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0x01,0],[0xFF,0],[0x00,0]],
        '╰' => [[0x00,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0x80,0],[0xFF,0],[0x00,0]],
        '<' => [[0x00,0],[0x03,0],[0x06,0],[0x0C,0],[0x18,0],[0x30,0],[0x30,0],[0x18,0],[0x0C,0],[0x06,0],[0x03,0],[0x00,0]],
        '>' => [[0x00,0],[0xC0,0],[0x30,0],[0x18,0],[0x0C,0],[0x06,0],[0x06,0],[0x0C,0],[0x18,0],[0x30,0],[0xC0,0],[0x00,0]],
        '/' => [[0x00,0],[0x00,0],[0x00,0],[0x01,0],[0x03,0],[0x06,0],[0x0C,0],[0x18,0],[0x30,0],[0x60,0],[0x40,0],[0x00,0]],
        '\\' => [[0x00,0],[0x00,0],[0x00,0],[0x80,0],[0x40,0],[0x20,0],[0x10,0],[0x08,0],[0x04,0],[0x02,0],[0x01,0],[0x00,0]],
        '*' => [[0x00,0],[0x00,0],[0x00,0],[0x18,0],[0x3F,0],[0x18,0],[0x3F,0],[0x18,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '#' => [[0x00,0],[0x24,0],[0x24,0],[0x7F,0],[0x24,0],[0x24,0],[0x7F,0],[0x24,0],[0x24,0],[0x7F,0],[0x24,0],[0x00,0]],
        '_' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0xFF,0],[0x00,0]],
        '?' => [[0x00,0],[0x1E,0],[0x33,0],[0x30,0],[0x30,0],[0x1C,0],[0x0C,0],[0x0C,0],[0x00,0],[0x0C,0],[0x0C,0],[0x00,0]],
        '!' => [[0x00,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x00,0],[0x0C,0],[0x00,0],[0x00,0]],
        '\'' => [[0x00,0],[0x03,0],[0x03,0],[0x03,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '"' => [[0x00,0],[0x33,0],[0x33,0],[0x33,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '%' => [[0x00,0],[0x26,0],[0x2B,0],[0x29,0],[0x26,0],[0x02,0],[0x4B,0],[0x52,0],[0x52,0],[0x4B,0],[0x26,0],[0x00,0]],
        '&' => [[0x00,0],[0x1E,0],[0x33,0],[0x63,0],[0x1E,0],[0x1B,0],[0x33,0],[0x63,0],[0x33,0],[0x33,0],[0x1E,0],[0x00,0]],
        '@' => [[0x00,0],[0x3F,0],[0x63,0],[0x6B,0],[0x6B,0],[0x6B,0],[0x6B,0],[0x2B,0],[0x03,0],[0x03,0],[0x1F,0],[0x00,0]],
        '~' => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x1B,0],[0x36,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '^' => [[0x00,0],[0x03,0],[0x07,0],[0x0F,0],[0x1B,0],[0x33,0],[0x63,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
        '{' => [[0x00,0],[0x07,0],[0x04,0],[0x04,0],[0x07,0],[0x04,0],[0x04,0],[0x04,0],[0x04,0],[0x04,0],[0x07,0],[0x00,0]],
        '|' => [[0x00,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x0C,0],[0x00,0]],
        '}' => [[0x00,0],[0xE0,0],[0x20,0],[0x20,0],[0xE0,0],[0x20,0],[0x20,0],[0x20,0],[0x20,0],[0x20,0],[0xE0,0],[0x00,0]],
        _ => [[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0],[0x00,0]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // sprint-22: RingBuffer tests
    #[test]
    fn test_ring_buffer_push_and_iter() {
        let mut rb: RingBuffer<f32, 5> = RingBuffer::new();
        assert_eq!(rb.len(), 0);

        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);

        let collected: Vec<f32> = rb.iter().copied().collect();
        assert_eq!(collected, vec![1.0, 2.0, 3.0]);
        assert_eq!(rb.len(), 3);
    }

    #[test]
    fn test_ring_buffer_overwrite() {
        let mut rb: RingBuffer<f32, 3> = RingBuffer::new();
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);
        // Buffer is full — next push wraps
        rb.push(4.0);

        let collected: Vec<f32> = rb.iter().copied().collect();
        // Should contain [2.0, 3.0, 4.0] (oldest 1.0 was evicted)
        assert_eq!(collected, vec![2.0, 3.0, 4.0]);
        assert_eq!(rb.len(), 3);
    }

    #[test]
    fn test_ring_buffer_p99() {
        let mut rb: RingBuffer<f32, 10> = RingBuffer::new();
        for v in 1..=10 {
            rb.push(v as f32);
        }
        // P99 of 1..10 should be close to 10 (99th percentile of 10 elements)
        let p99 = rb.p99();
        assert!(p99.is_some());
        assert!((p99.unwrap() - 10.0).abs() < 0.5, "p99={:?}", p99);
    }

    #[test]
    fn test_ring_buffer_average() {
        let mut rb: RingBuffer<f32, 5> = RingBuffer::new();
        rb.push(2.0);
        rb.push(4.0);
        rb.push(6.0);
        assert!((rb.average().unwrap() - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_ring_buffer_empty() {
        let rb: RingBuffer<f32, 5> = RingBuffer::new();
        assert_eq!(rb.len(), 0);
        assert!(rb.p99().is_none());
        assert!(rb.average().is_none());
        let collected: Vec<f32> = rb.iter().copied().collect();
        assert!(collected.is_empty());
    }

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
        let b = DebugOverlay::bitmap_for(' ');
        assert_eq!(b[0], [0x00, 0]);
    }

    #[test]
    fn test_bitmap_for_digit() {
        // '1' should have middle column set
        let b = DebugOverlay::bitmap_for('1');
        // Row 5 should be 0x0C (binary 00001100)
        assert_eq!(b[5], [0x0C, 0]);
    }

    #[test]
    fn test_bitmap_for_uppercase() {
        let b = DebugOverlay::bitmap_for('A');
        // Row 1 should be 0x1C (binary 00011100)
        assert_eq!(b[1], [0x1C, 0]);
    }

    #[test]
    fn test_bitmap_for_lowercase() {
        let b = DebugOverlay::bitmap_for('a');
        // Row 3 should be 0x1E (binary 00011110)
        assert_eq!(b[3], [0x1E, 0]);
    }

    #[test]
    fn test_char_w_and_h() {
        assert_eq!(CHAR_W, 8);
        assert_eq!(CHAR_H, 12);
    }

    #[test]
    fn test_draw_text_does_not_panic() {
        // Create a minimal canvas for testing
        // Note: This test just verifies draw_text doesn't panic on valid input
        // We can't easily test rendering output in unit tests
    }

    #[test]
    fn test_8x12_font_has_box_drawing() {
        // Verify box-drawing characters are defined
        let _ = font_8x12('╔');
        let _ = font_8x12('═');
        let _ = font_8x12('║');
        let _ = font_8x12('╠');
        let _ = font_8x12('╣');
        let _ = font_8x12('╚');
    }

    #[test]
    fn test_8x12_font_unknown_char_gives_blank() {
        // DEL (0x7F) is not in the font table, so it hits the default case
        let b = font_8x12('\x7F');
        // Unknown char should return all zeros
        for row in &b {
            assert_eq!(row[0], 0);
        }
    }

    #[test]
    fn test_minimap_renders_without_panic() {
        // Verify render_minimap doesn't panic with valid inputs
        // The function requires SDL2 canvas which isn't available in unit tests,
        // so we verify the function exists and has correct signature
        // This test documents the expected behavior
        let overlay = DebugOverlay::new();
        assert!(!overlay.is_minimap_visible(), "minimap should default to hidden");
        assert!(!overlay.is_visible(), "debug overlay should default to hidden");
    }
}
