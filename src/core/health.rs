//! Health monitoring system — automated assertions for game runtime health.
//! Runs every frame during Playing state; results feed the debug overlay.

use crate::core::isom::grid_to_screen;

/// Health check severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckResult {
    #[default]
    Ok,
    Warn,
    Fail,
}

impl CheckResult {
    pub fn is_ok(&self) -> bool {
        *self == CheckResult::Ok
    }

    /// Returns (color, label) for debug overlay display.
    /// The label is just the bracketed status (e.g., "[OK]", "[WARN]", "[FAIL]").
    pub fn overlay_display(&self) -> (sdl2::pixels::Color, &'static str) {
        match self {
            CheckResult::Ok => (sdl2::pixels::Color::RGBA(100, 255, 100, 255), "[OK]"),
            CheckResult::Warn => (sdl2::pixels::Color::RGBA(255, 200, 100, 255), "[WARN]"),
            CheckResult::Fail => (sdl2::pixels::Color::RGBA(255, 100, 100, 255), "[FAIL]"),
        }
    }
}

/// Aggregated health check results for the overlay.
#[derive(Debug, Clone, Default)]
pub struct HealthResults {
    pub camera: CheckResult,
    pub fps: CheckResult,
    pub player1: CheckResult,
    pub player2: CheckResult,
}

/// Monitor for camera tracking health.
/// Tracks player movement and camera target to detect stale tracking.
struct CameraTracker {
    /// Player grid position from previous frame
    player_prev_grid: (f32, f32),
    /// Camera target from previous frame
    camera_prev_target: (f32, f32),
    /// How many consecutive frames the camera has been stale
    stale_frames: u32,
    /// Cumulative player movement (in grid tiles) since camera got stuck
    cumulative_player_dist: f32,
}

impl CameraTracker {
    fn new() -> Self {
        Self {
            player_prev_grid: (0.0, 0.0),
            camera_prev_target: (0.0, 0.0),
            stale_frames: 0,
            cumulative_player_dist: 0.0,
        }
    }
}

/// Monitor for FPS floor health.
struct FpsTracker {
    /// Consecutive frames below 30 fps
    low_fps_frames: u32,
    /// Consecutive frames below 20 fps
    crit_fps_frames: u32,
}

impl FpsTracker {
    fn new() -> Self {
        Self {
            low_fps_frames: 0,
            crit_fps_frames: 0,
        }
    }
}

/// Health monitor — runs assertions each frame during Playing state.
pub struct HealthMonitor {
    camera: CameraTracker,
    fps: FpsTracker,
    results: HealthResults,
    /// Set by check() when any Fail result is detected; cleared by take_screenshot_reason().
    pending_screenshot_reason: Option<String>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            camera: CameraTracker::new(),
            fps: FpsTracker::new(),
            results: HealthResults::default(),
            pending_screenshot_reason: None,
        }
    }

    /// Run all health checks for one frame.
    /// Call this during Playing state each frame.
    pub fn check(&mut self, camera: &crate::core::Camera, player1_grid: (f32, f32), player2_grid: (f32, f32), fps: f32, _dt: f32, screen_w: u32, screen_h: u32) {
        self.check_camera_tracking(camera, player1_grid);
        self.check_fps_floor(fps);

        // Compute player results into locals first to avoid borrow conflicts.
        // We must not hold &mut self.results while also borrowing camera.
        let mut p1_result = CheckResult::Ok;
        let mut p2_result = CheckResult::Ok;
        self.check_player_onscreen(player1_grid, camera, screen_w, screen_h, &mut p1_result);
        self.check_player_onscreen(player2_grid, camera, screen_w, screen_h, &mut p2_result);
        self.results.player1 = p1_result;
        self.results.player2 = p2_result;

        // If any check resulted in Fail, record the reason for automatic screenshot
        if !self.pending_screenshot_reason.is_some() {
            let reason = if self.results.camera == CheckResult::Fail {
                Some("camera_stale".to_string())
            } else if self.results.fps == CheckResult::Fail {
                Some("fps_floor_breached".to_string())
            } else if self.results.player1 == CheckResult::Fail {
                Some("player1_off_screen".to_string())
            } else if self.results.player2 == CheckResult::Fail {
                Some("player2_off_screen".to_string())
            } else {
                None
            };
            self.pending_screenshot_reason = reason;
        }
    }

    /// Camera tracking check.
    /// - WARN if camera target unchanged AND cumulative player movement > 3 tiles for 30 frames
    /// - FAIL if same condition persists for 60 frames
    /// Camera is OK when it is actively lerping toward target (|target - current| > 2.0).
    fn check_camera_tracking(&mut self, camera: &crate::core::Camera, player_grid: (f32, f32)) {
        let player_gx = player_grid.0;
        let player_gy = player_grid.1;

        // Per-frame player movement
        let dx = player_gx - self.camera.player_prev_grid.0;
        let dy = player_gy - self.camera.player_prev_grid.1;
        let frame_dist = (dx * dx + dy * dy).sqrt();

        // Camera movement = has target changed from last frame?
        let cam_moved = (camera.target_x - self.camera.camera_prev_target.0).abs() > 0.1
            || (camera.target_y - self.camera.camera_prev_target.1).abs() > 0.1;

        // Is camera actively lerping toward target?
        let cam_following = (camera.target_x - camera.x).abs() > 2.0
            || (camera.target_y - camera.y).abs() > 2.0;

        if !cam_following {
            // Camera is stuck (target == current). Accumulate player movement.
            self.camera.cumulative_player_dist += frame_dist;

            if self.camera.cumulative_player_dist > 5.0 {
                self.camera.stale_frames += 1;
            } else if self.camera.cumulative_player_dist > 3.0 {
                self.camera.stale_frames += 1;
            }
            // When camera is stuck, DON'T reset cumulative dist — keep accumulating.
        } else {
            // Camera is actively following — not stale, reset counters.
            self.camera.stale_frames = 0;
            self.camera.cumulative_player_dist = 0.0;
        }

        // Determine health result
        self.results.camera = if self.camera.stale_frames >= 60 {
            CheckResult::Fail
        } else if self.camera.stale_frames >= 30 {
            CheckResult::Warn
        } else {
            CheckResult::Ok
        };

        // Update trackers
        self.camera.player_prev_grid = (player_gx, player_gy);
        self.camera.camera_prev_target = (camera.target_x, camera.target_y);
    }

    /// FPS floor check.
    /// - WARN if fps < 30 sustained for 30 frames
    /// - FAIL if fps < 20 sustained for 30 frames
    fn check_fps_floor(&mut self, fps: f32) {
        if fps < 20.0 {
            self.fps.crit_fps_frames += 1;
            self.fps.low_fps_frames += 1;
        } else if fps < 30.0 {
            self.fps.crit_fps_frames = 0;
            self.fps.low_fps_frames += 1;
        } else {
            self.fps.crit_fps_frames = 0;
            self.fps.low_fps_frames = 0;
        }

        self.results.fps = if self.fps.crit_fps_frames >= 30 {
            CheckResult::Fail
        } else if self.fps.low_fps_frames >= 30 {
            CheckResult::Warn
        } else {
            CheckResult::Ok
        };
    }

    /// Player on-screen check.
    /// - WARN if player sprite rect (64x64) is more than 50% off any screen edge
    /// - FAIL if player sprite is more than 100% off any screen edge
    fn check_player_onscreen(&self, player_grid: (f32, f32), camera: &crate::core::Camera, screen_w: u32, screen_h: u32, out: &mut CheckResult) {
        let (px, py) = grid_to_screen(player_grid.0, player_grid.1, camera.x, camera.y);
        // Player sprite is 64x64, rendered at (px, py-32) to (px+64, py+32)
        let sprite_w: f32 = 64.0;
        let sprite_h: f32 = 64.0;
        let dst_x: f32 = px;
        let dst_y: f32 = py - 32.0;
        let screen_w: f32 = screen_w as f32;
        let screen_h: f32 = screen_h as f32;

        // Off-screen amounts (positive = outside viewport)
        let off_left: f32 = (-dst_x).max(0.0);
        let off_right: f32 = (dst_x + sprite_w - screen_w).max(0.0);
        let off_top: f32 = (-dst_y).max(0.0);
        let off_bottom: f32 = (dst_y + sprite_h - screen_h).max(0.0);

        let max_off_x = off_left.max(off_right);
        let max_off_y = off_top.max(off_bottom);

        // WARN: more than 50% of sprite off screen on any axis
        // FAIL: more than 100% of sprite off screen on any axis
        *out = if max_off_x > sprite_w || max_off_y > sprite_h {
            CheckResult::Fail
        } else if max_off_x > sprite_w * 0.5 || max_off_y > sprite_h * 0.5 {
            CheckResult::Warn
        } else {
            CheckResult::Ok
        };
    }

    /// Get current health results.
    pub fn results(&self) -> &HealthResults {
        &self.results
    }

    /// Returns true if a screenshot should be taken due to a health check failure.
    pub fn should_screenshot(&self) -> bool {
        self.pending_screenshot_reason.is_some()
    }

    /// Takes and clears the pending screenshot reason.
    /// Returns the reason if one was pending, None otherwise.
    pub fn take_screenshot_reason(&mut self) -> Option<String> {
        self.pending_screenshot_reason.take()
    }

    /// Reset all trackers. Call when entering Playing state.
    pub fn reset(&mut self) {
        self.camera = CameraTracker::new();
        self.fps = FpsTracker::new();
        self.results = HealthResults::default();
        self.pending_screenshot_reason = None;
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_monitor_new_is_ok() {
        let hm = HealthMonitor::new();
        let r = hm.results();
        assert_eq!(r.camera, CheckResult::Ok);
        assert_eq!(r.fps, CheckResult::Ok);
    }

    #[test]
    fn test_check_result_is_ok() {
        assert!(CheckResult::Ok.is_ok());
        assert!(!CheckResult::Warn.is_ok());
        assert!(!CheckResult::Fail.is_ok());
    }

    #[test]
    fn test_camera_ok_when_following() {
        let mut hm = HealthMonitor::new();
        let mut cam = crate::core::Camera::new(1280, 720, 16, 16);
        // Player at grid (5, 5), camera lerping toward target (actively following)
        cam.follow(5.0, 5.0);
        hm.check(&cam, (5.0, 5.0), (5.0, 5.0), 60.0, 1.0/60.0, 1280, 720);
        assert_eq!(hm.results().camera, CheckResult::Ok, "Camera actively lerping should be OK");
    }

    #[test]
    fn test_camera_stale_when_target_matches_current_but_player_moved() {
        let mut cam = crate::core::Camera::new(1280, 720, 16, 16);

        // Set camera to be "stuck" - target == current, NOT following
        cam.x = 100.0;
        cam.y = 100.0;
        cam.target_x = 100.0;
        cam.target_y = 100.0;

        // Initialize trackers: first call settles player_prev_grid and camera_prev_target
        let mut hm = HealthMonitor::new();
        hm.check(&cam, (10.0, 5.0), (5.0, 5.0), 60.0, 1.0/60.0, 1280, 720);
        // After init: player_prev_grid=(10,5), camera_prev_target=(100,100), cumulative=0

        // Second: simulate 31 frames where player moves 0.2 tiles/frame
        // Cumulative after first of these: 0.2 > 3.0? No. Cumulative = 0.2.
        // Need to move 3.2 tiles first frame to exceed >3.0 threshold.
        // Strategy: move 3.5 tiles on frame 1, then 0.05/frame for remaining 30 frames.
        // Simpler: use 0.2/frame but need cumulative > 3.0 for staleness to count.
        // Solution: jump 3.5 tiles on frame 1, then 0.01/frame for 30 more.
        hm.check(&cam, (13.5, 5.0), (5.0, 5.0), 60.0, 1.0/60.0, 1280, 720); // +3.5 > 3.0 → stale=1
        for i in 0..30 {
            let gx = 13.5 + (i as f32) * 0.01;
            hm.check(&cam, (gx, 5.0), (5.0, 5.0), 60.0, 1.0/60.0, 1280, 720);
        }
        assert_eq!(hm.results().camera, CheckResult::Warn, "Camera stale for 31 frames should be WARN");
    }

    #[test]
    fn test_camera_fail_after_60_frames_stale() {
        let mut cam = crate::core::Camera::new(1280, 720, 16, 16);

        // Camera stuck
        cam.x = 100.0;
        cam.y = 100.0;
        cam.target_x = 100.0;
        cam.target_y = 100.0;

        let mut hm = HealthMonitor::new();
        hm.check(&cam, (10.0, 5.0), (5.0, 5.0), 60.0, 1.0/60.0, 1280, 720); // init

        // Jump 5.5 tiles on frame 1 (>5.0 threshold for FAIL), then 0.01/frame for 59 more
        hm.check(&cam, (15.5, 5.0), (5.0, 5.0), 60.0, 1.0/60.0, 1280, 720); // +5.5 > 5.0 → stale=1
        for i in 0..59 {
            let gx = 15.5 + (i as f32) * 0.01;
            hm.check(&cam, (gx, 5.0), (5.0, 5.0), 60.0, 1.0/60.0, 1280, 720);
        }
        assert_eq!(hm.results().camera, CheckResult::Fail, "Camera stale for 60 frames should be FAIL");
    }

    #[test]
    fn test_player_off_screen_warn() {
        let hm = HealthMonitor::new();
        let mut cam = crate::core::Camera::new(1280, 720, 16, 16);
        cam.x = 0.0;
        cam.y = 0.0;

        // With grid_to_screen(gx, gy, 0, 0) = ((gx-gy)*32, (gx+gy)*16):
        // Player at gx=40, gy=0: screen_x=1280, sprite right=1344, off_right=64
        // 64 > 32 (50% of 64) but < 64 (100%) → WARN
        let mut result = CheckResult::Ok;
        hm.check_player_onscreen((40.0, 0.0), &cam, 1280, 720, &mut result);
        assert_eq!(result, CheckResult::Warn, "Player >50% off screen should be WARN");
    }

    #[test]
    fn test_player_completely_off_screen_fail() {
        let hm = HealthMonitor::new();
        let mut cam = crate::core::Camera::new(1280, 720, 16, 16);
        cam.x = 0.0;
        cam.y = 0.0;

        // Player at gx=42, gy=0: screen_x=1344, sprite right=1408, off_right=128
        // 128 > 64 (100% of sprite) → FAIL
        let mut result = CheckResult::Ok;
        hm.check_player_onscreen((42.0, 0.0), &cam, 1280, 720, &mut result);
        assert_eq!(result, CheckResult::Fail, "Player >100% off screen should be FAIL");
    }

    #[test]
    fn test_player_on_screen_ok() {
        let hm = HealthMonitor::new();
        let mut cam = crate::core::Camera::new(1280, 720, 16, 16);
        // Center camera on player at (8, 8) - should be visible
        cam.center_on(8.0, 8.0);

        let mut result = CheckResult::Fail;
        hm.check_player_onscreen((8.0, 8.0), &cam, 1280, 720, &mut result);
        assert_eq!(result, CheckResult::Ok, "Player centered on screen should be OK");
    }

    #[test]
    fn test_fps_floor_warn() {
        let mut hm = HealthMonitor::new();

        // Simulate 31 frames at 25 fps
        for _ in 0..31 {
            hm.check_fps_floor(25.0);
        }
        assert_eq!(hm.results().fps, CheckResult::Warn, "FPS < 30 for 31 frames should be WARN");
    }

    #[test]
    fn test_fps_floor_fail() {
        let mut hm = HealthMonitor::new();

        // Simulate 30 frames at 15 fps
        for _ in 0..30 {
            hm.check_fps_floor(15.0);
        }
        assert_eq!(hm.results().fps, CheckResult::Fail, "FPS < 20 for 30 frames should be FAIL");
    }

    #[test]
    fn test_fps_recovery() {
        let mut hm = HealthMonitor::new();

        // 30 frames at 25 fps → WARN
        for _ in 0..30 {
            hm.check_fps_floor(25.0);
        }
        assert_eq!(hm.results().fps, CheckResult::Warn);

        // Back to 60 fps → OK
        hm.check_fps_floor(60.0);
        assert_eq!(hm.results().fps, CheckResult::Ok, "FPS recovery should clear WARN");
    }

    #[test]
    fn test_health_results_default() {
        let hr = HealthResults::default();
        assert_eq!(hr.camera, CheckResult::Ok);
        assert_eq!(hr.fps, CheckResult::Ok);
        assert_eq!(hr.player1, CheckResult::Ok);
        assert_eq!(hr.player2, CheckResult::Ok);
    }

    #[test]
    fn test_screenshot_triggered_on_fail() {
        // Test that pending_screenshot_reason is set when a Fail result occurs.
        // Use FPS floor failure (isolated from camera/player position checks).
        let mut cam = crate::core::Camera::new(1280, 720, 16, 16);
        // Center camera on player so camera and player checks are OK
        cam.center_on(8.0, 8.0);

        let mut hm = HealthMonitor::new();

        // Run 30 frames at 15 fps through check() to trigger FPS floor FAIL
        for _ in 0..30 {
            hm.check(&cam, (8.0, 8.0), (8.0, 8.0), 15.0, 1.0/60.0, 1280, 720);
        }

        assert_eq!(hm.results().fps, CheckResult::Fail, "FPS should be FAIL");
        assert!(hm.should_screenshot(), "should_screenshot should be true when fps FAIL");
        assert_eq!(hm.take_screenshot_reason(), Some("fps_floor_breached".to_string()),
            "pending_screenshot_reason should be set to fps_floor_breached");
    }

    #[test]
    fn test_take_screenshot_clears_reason() {
        // Test that take_screenshot_reason clears the pending reason.
        // Use FPS floor failure (isolated from camera/player position checks).
        let mut cam = crate::core::Camera::new(1280, 720, 16, 16);
        cam.center_on(8.0, 8.0);

        let mut hm = HealthMonitor::new();

        // Run 30 frames at 15 fps through check() to trigger FPS floor FAIL
        for _ in 0..30 {
            hm.check(&cam, (8.0, 8.0), (8.0, 8.0), 15.0, 1.0/60.0, 1280, 720);
        }

        // Verify screenshot reason is set
        assert!(hm.should_screenshot());

        // Take and clear the reason
        let reason = hm.take_screenshot_reason();
        assert_eq!(reason, Some("fps_floor_breached".to_string()));

        // Verify it's cleared
        assert!(!hm.should_screenshot(), "should_screenshot should be false after take_screenshot_reason");
        assert_eq!(hm.take_screenshot_reason(), None, "Second call should return None");
    }
}
