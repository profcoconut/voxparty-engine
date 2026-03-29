use crate::core::isom::{TILE_W, TILE_H, grid_to_screen};

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
            lerp_speed: 0.1,
        }
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
    }
}
