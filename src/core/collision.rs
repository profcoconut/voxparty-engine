use crate::core::isom::DIRECTIONS;

/// Try to move a grid entity in the given direction.
/// Returns the new position if valid, or None if blocked.
pub fn try_move(
    from_x: i32,
    from_y: i32,
    dir_idx: usize,
    world: &impl WorldAccess,
) -> Option<(i32, i32)> {
    let (dx, dy) = DIRECTIONS[dir_idx];
    let to_x = from_x + dx;
    let to_y = from_y + dy;

    // Out of bounds is allowed (player falls off)
    if to_x < 0 || to_y < 0 || to_x >= world.grid_w() || to_y >= world.grid_h() {
        return Some((to_x, to_y));
    }

    // Solid tiles block movement
    if world.is_solid(to_x, to_y) {
        return None;
    }

    Some((to_x, to_y))
}

/// Trait for world access during collision checks.
pub trait WorldAccess {
    fn grid_w(&self) -> i32;
    fn grid_h(&self) -> i32;
    fn is_solid(&self, x: i32, y: i32) -> bool;
}
