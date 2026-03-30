/// Classic 2:1 isometric projection.
/// Tile size: 32x16 pixels (width : height = 2 : 1)
pub const TILE_WIDTH: f32 = 32.0;
pub const TILE_HEIGHT: f32 = 16.0;

/// Convert grid coordinates to screen coordinates.
pub fn grid_to_screen(x: i32, y: i32) -> (f32, f32) {
    let sx = (x - y) as f32 * (TILE_WIDTH / 2.0);
    let sy = (x + y) as f32 * (TILE_HEIGHT / 2.0);
    (sx, sy)
}

/// Convert screen coordinates to grid coordinates.
pub fn screen_to_grid(sx: f32, sy: f32, _cam_x: f32, _cam_y: f32) -> (i32, i32) {
    let gx = ((sx / (TILE_WIDTH / 2.0)) + (sy / (TILE_HEIGHT / 2.0))) as i32 / 2;
    let gy = ((sy / (TILE_HEIGHT / 2.0)) - (sx / (TILE_WIDTH / 2.0))) as i32 / 2;
    (gx, gy)
}
