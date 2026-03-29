/// Isometric coordinate conversion utilities.
/// Classic 2:1 isometric projection (tile_w:tile_h = 2:1 ratio).
pub const TILE_W: f32 = 64.0;
pub const TILE_H: f32 = 32.0;

/// Convert grid (gx, gy) to screen (px, py) position.
/// Camera offsets (cam_x, cam_y) pan the view in screen pixels.
pub fn grid_to_screen(gx: f32, gy: f32, cam_x: f32, cam_y: f32) -> (f32, f32) {
    let px = (gx - gy) * (TILE_W / 2.0) - cam_x;
    let py = (gx + gy) * (TILE_H / 2.0) - cam_y;
    (px, py)
}

/// Convert screen (px, py) back to floating-point grid coordinates.
pub fn screen_to_grid(px: f32, py: f32, cam_x: f32, cam_y: f32) -> (f32, f32) {
    let px = px + cam_x;
    let py = py + cam_y;
    let gx = (px / TILE_W + py / TILE_H) / 2.0;
    let gy = (px / TILE_W - py / TILE_H) / 2.0;
    (gx, gy)
}

/// Depth key for sorting: lower depth is drawn first.
/// Z=0 for floor tiles, z=1 for elevated sprites.
pub fn depth_key(gx: i32, gy: i32, z: i32) -> i32 {
    gx + gy + z * 1000
}

/// The 8 cardinal + diagonal directions on the isometric grid.
/// Index maps to (dx, dy) in grid space.
/// 0=N(up-left), 1=NE, 2=E(down-right), 3=SE, 4=S(down-left), 5=SW, 6=W(up-right), 7=NW
pub const DIRECTIONS: [(i32, i32); 8] = [
    (0, -1),  // N
    (1, -1),  // NE
    (1, 0),   // E
    (1, 1),   // SE
    (0, 1),   // S
    (-1, 1),  // SW
    (-1, 0),  // W
    (-1, -1), // NW
];

/// Quantize an analog joystick (-1..1 each axis) to an 8-way direction index.
/// Returns None if the input is within the deadzone.
pub fn quantize_direction(jx: f32, jy: f32) -> Option<usize> {
    let deadzone = 0.3;
    if jx.abs() < deadzone && jy.abs() < deadzone {
        return None;
    }
    // atan2 gives angle from positive X axis, counterclockwise
    // We want angle from positive Y axis (screen down), clockwise
    let angle = jy.atan2(jx); // radians
    // Map to 0-7 sector: sector 0 = N, sectors clockwise
    let sector = ((angle + std::f32::consts::PI) / (std::f32::consts::TAU) * 8.0) as i32;
    let dir = ((sector % 8) + 8) % 8;
    Some(dir as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_to_screen_forward() {
        // Test forward transformation: (5,3) → screen
        let (px, py) = grid_to_screen(5.0, 3.0, 0.0, 0.0);
        assert!((px - 64.0).abs() < 0.001, "px = {}", px);
        assert!((py - 128.0).abs() < 0.001, "py = {}", py);
    }

    #[test]
    fn test_quantize_deadzone() {
        assert_eq!(quantize_direction(0.1, 0.1), None);
    }

    #[test]
    fn test_quantize_north() {
        // Pure up (negative Y in screen = north)
        let dir = quantize_direction(0.0, -1.0);
        assert!(dir.is_some());
    }

    #[test]
    fn test_depth_key_ordering() {
        // Items with lower depth are drawn first
        assert!(depth_key(0, 0, 0) < depth_key(1, 0, 0));
        assert!(depth_key(0, 0, 0) < depth_key(0, 1, 0));
        // Z=1 (elevated) should always sort after z=0
        assert!(depth_key(0, 0, 1) > depth_key(100, 100, 0));
    }
}
