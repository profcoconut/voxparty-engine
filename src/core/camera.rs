use crate::core::isom::{TILE_W, TILE_H, grid_to_screen};

#[derive(Debug)]
pub struct Camera {
    /// Current pixel offset (top-left of viewport in world space)
    pub x: f32,
    pub y: f32,
    /// Target pixel offset (lerped toward)
    pub target_x: f32,
    pub target_y: f32,
    /// Screen dimensions in pixels
    screen_w: u32,
    screen_h: u32,
    /// World dimensions in grid units
    world_w: i32,
    world_h: i32,
    /// Deadzone fraction (0.0 = snap to player, 0.4 = 40% center region)
    pub deadzone: f32,
    /// Lerp speed toward target (0.0..1.0)
    pub lerp_speed: f32,
    /// Screen shake timer (seconds remaining)
    shake_timer: f32,
    /// Screen shake intensity (pixel offset magnitude)
    shake_intensity: f32,
}

impl Camera {
    pub fn new(screen_w: u32, screen_h: u32, world_w: i32, world_h: i32) -> Self {
        // Start camera centered on the world
        let world_center_gx = world_w as f32 / 2.0;
        let world_center_gy = world_h as f32 / 2.0;
        let (cx, cy) = grid_to_screen(world_center_gx, world_center_gy, 0.0, 0.0);
        let offset_x = cx - screen_w as f32 / 2.0;
        let offset_y = cy - screen_h as f32 / 2.0;

        Self {
            x: offset_x,
            y: offset_y,
            target_x: offset_x,
            target_y: offset_y,
            screen_w,
            screen_h,
            world_w,
            world_h,
            deadzone: 0.2,
            lerp_speed: 0.15,
            shake_timer: 0.0,
            shake_intensity: 0.0,
        }
    }

    /// Instantly center the camera on the given grid position.
    /// Call this when entering Playing state to ensure player is visible.
    pub fn center_on(&mut self, gx: f32, gy: f32) {
        let (px, py) = grid_to_screen(gx, gy, 0.0, 0.0);
        // Camera should position so player is centered on screen
        let target_screen_x = px + TILE_W / 2.0;
        let target_screen_y = py + TILE_H / 2.0;
        self.target_x = target_screen_x - self.screen_w as f32 / 2.0;
        self.target_y = target_screen_y - self.screen_h as f32 / 2.0;
        // Clamp to world bounds
        let max_cam_x = self.world_w as f32 * TILE_W - self.screen_w as f32;
        let max_cam_y = self.world_h as f32 * TILE_H - self.screen_h as f32;
        self.target_x = self.target_x.clamp(0.0, max_cam_x.max(0.0));
        self.target_y = self.target_y.clamp(0.0, max_cam_y.max(0.0));
        // Snap current position to target
        self.x = self.target_x;
        self.y = self.target_y;
    }

    /// Update camera to follow the given grid position.
    pub fn follow(&mut self, gx: f32, gy: f32) {
        // Convert grid target to screen position
        let (px, py) = grid_to_screen(gx, gy, 0.0, 0.0);
        // Where should the camera be to center this grid tile on screen?
        let target_screen_x = px + TILE_W / 2.0;
        let target_screen_y = py + TILE_H / 2.0;

        // Deadzone: only move if player is outside the center region
        let screen_cx = self.screen_w as f32 / 2.0;
        let screen_cy = self.screen_h as f32 / 2.0;
        let margin_x = screen_cx * self.deadzone;
        let margin_y = screen_cy * self.deadzone;

        let dx = target_screen_x - screen_cx;
        let dy = target_screen_y - screen_cy;

        if dx < -margin_x {
            self.target_x = target_screen_x - screen_cx + margin_x;
        } else if dx > margin_x {
            self.target_x = target_screen_x - screen_cx - margin_x;
        }

        if dy < -margin_y {
            self.target_y = target_screen_y - screen_cy + margin_y;
        } else if dy > margin_y {
            self.target_y = target_screen_y - screen_cy - margin_y;
        }

        // Clamp to world bounds
        let max_cam_x = self.world_w as f32 * TILE_W - self.screen_w as f32;
        let max_cam_y = self.world_h as f32 * TILE_H - self.screen_h as f32;
        self.target_x = self.target_x.clamp(0.0, max_cam_x.max(0.0));
        self.target_y = self.target_y.clamp(0.0, max_cam_y.max(0.0));

        // Lerp toward target
        self.x += (self.target_x - self.x) * self.lerp_speed;
        self.y += (self.target_y - self.y) * self.lerp_speed;

        // Apply screen shake (overrides camera position for a few frames)
        if self.shake_timer > 0.0 {
            let shake_x = (rand_simple() * 2.0 - 1.0) * self.shake_intensity;
            let shake_y = (rand_simple() * 2.0 - 1.0) * self.shake_intensity;
            self.x += shake_x;
            self.y += shake_y;
        }
    }

    /// Trigger a screen shake effect.
    /// `intensity` is the maximum pixel offset magnitude.
    /// `duration` is how long the shake lasts in seconds.
    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.shake_intensity = intensity;
        self.shake_timer = duration;
    }

    /// Decay the screen shake timer by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        if self.shake_timer > 0.0 {
            self.shake_timer -= dt;
            if self.shake_timer < 0.0 {
                self.shake_timer = 0.0;
            }
        }
    }
}

/// Simple deterministic pseudo-random (for shake jitter, no external dependency).
fn rand_simple() -> f32 {
    // Uses the system time or a counter for slight variation
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos as f32 * 0.618034).fract()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_centers_camera() {
        // 16x16 grid world, 1280x720 screen
        let cam = Camera::new(1280, 720, 16, 16);
        // Camera starts at world_center - screen_center
        // grid_to_screen(8, 8) = (0, 256); offset = (0-640, 256-360) = (-640, -104)
        assert_eq!(cam.x, -640.0, "camera x = world_center - screen_w/2");
        assert_eq!(cam.y, -104.0, "camera y = world_center - screen_h/2");
        // Target should match actual (no lerp needed at start)
        assert_eq!(cam.x, cam.target_x);
        assert_eq!(cam.y, cam.target_y);
    }

    #[test]
    fn test_follow_moves_camera_toward_target() {
        let mut cam = Camera::new(1280, 720, 16, 16);
        let initial_x = cam.x;
        // Follow a position far from current center
        cam.follow(0.0, 0.0);
        // Camera should have moved (lerp speed = 0.1, so partially toward target)
        assert_ne!(cam.x, initial_x, "camera x should move after follow()");
    }

    #[test]
    fn test_follow_lerp_speed() {
        // Test that camera lerps toward target, not snapping
        let mut cam = Camera::new(1280, 720, 32, 32);
        cam.lerp_speed = 0.5;
        // Initial camera at world center: x=-640, y=152
        // follow(0,0) sets clamped target to (0,0)
        // x lerps: -640 + (0+640)*0.5 = -320
        // y lerps: 152 + (0-152)*0.5 = 76
        cam.follow(0.0, 0.0);
        assert_eq!(cam.x, -320.0, "x lerps halfway to target at lerp_speed=0.5");
        assert_eq!(cam.y, 76.0, "y lerps halfway to target at lerp_speed=0.5");
    }

    #[test]
    fn test_follow_clamps_to_world_bounds() {
        // World smaller than screen: camera can't show the full world
        let mut cam = Camera::new(1280, 720, 4, 4);
        cam.lerp_speed = 1.0;
        // Follow far beyond world edge
        cam.follow(100.0, 100.0);
        // Camera target should be clamped to world bounds
        let _max_x: f32 = 4.0 * 64.0 - 1280.0; // world smaller than screen
        let _max_y: f32 = 4.0 * 32.0 - 720.0;
        // With world smaller than screen, clamping puts camera at 0 (can't go negative)
        assert_eq!(cam.target_x, 0.0, "camera clamped to 0 when world < screen width");
        assert_eq!(cam.target_y, 0.0, "camera clamped to 0 when world < screen height");
    }

    #[test]
    fn test_shake_sets_timer_and_intensity() {
        let mut cam = Camera::new(1280, 720, 16, 16);
        cam.shake(5.0, 0.1);
        assert_eq!(cam.shake_intensity, 5.0);
        assert_eq!(cam.shake_timer, 0.1);
    }

    #[test]
    fn test_shake_timer_decay() {
        let mut cam = Camera::new(1280, 720, 16, 16);
        cam.shake(5.0, 0.1);
        cam.tick(0.05);
        assert!(cam.shake_timer > 0.0, "shake timer should decay but not be zero yet");
        cam.tick(0.05);
        assert_eq!(cam.shake_timer, 0.0, "shake timer should be zero after full decay");
    }

    #[test]
    fn test_shake_does_not_go_negative() {
        let mut cam = Camera::new(1280, 720, 16, 16);
        cam.shake(5.0, 0.1);
        cam.tick(0.2); // decay past zero
        assert_eq!(cam.shake_timer, 0.0, "shake timer should not go negative");
    }
}
