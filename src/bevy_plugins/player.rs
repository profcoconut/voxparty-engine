//! Player component and movement systems for VoxParty ECS migration.
//!
//! Converts the OOP `Player` struct from `game/player.rs` into a Bevy `PlayerComponent`.
//! Input flows from the `Players` resource (VirtualGamepad) into the player entity systems.
//!
//! Provides:
//! - `PlayerComponent`: Bevy component holding player state, grid position, checkpoint
//! - `PlayerState`: enum for player lifecycle (Idle, Walking, Eliminated, Won)
//! - `player_movement_system`: reads input, applies grid movement, updates GridPos
//! - `player_animation_system`: advances animation timers based on movement state
//! - `PlayerPlugin`: registers all player systems

use bevy::prelude::*;
use crate::core::collision::try_move;
use crate::core::quantize_direction;
use crate::bevy_plugins::input::{Players, GameInput, VirtualGamepad};
use crate::bevy_plugins::sprite::GridPos;

/// Player lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Idle,
    Walking,
    Eliminated,
    Won,
}

/// Bevy component replacing the OOP `Player` struct.
/// Each player entity in the ECS world has exactly one `PlayerComponent`.
#[derive(Component, Debug)]
pub struct PlayerComponent {
    /// Player number (1 or 2)
    pub player_id: u8,
    /// Current grid X position
    pub grid_x: i32,
    pub grid_y: i32,
    /// Last checkpoint grid position
    pub checkpoint_x: i32,
    pub checkpoint_y: i32,
    /// Current player state
    pub state: PlayerState,
    /// Movement cooldown timer in seconds (0 = can move)
    pub cooldown: f32,
    /// Facing direction index (0-7, maps to DIRECTIONS)
    pub facing_dir: usize,
    /// Number of times this player has respawned
    pub respawn_count: u32,
}

impl PlayerComponent {
    pub fn new(player_id: u8, x: i32, y: i32) -> Self {
        Self {
            player_id,
            grid_x: x,
            grid_y: y,
            checkpoint_x: x,
            checkpoint_y: y,
            state: PlayerState::Idle,
            cooldown: 0.0,
            facing_dir: 0,
            respawn_count: 0,
        }
    }
}

/// Get the 8-way direction index from held directional inputs.
/// Returns None if no direction is held.
fn held_to_direction(vg: &VirtualGamepad) -> Option<usize> {
    let left = vg.is_held(GameInput::MoveLeft);
    let right = vg.is_held(GameInput::MoveRight);
    let up = vg.is_held(GameInput::MoveUp);
    let down = vg.is_held(GameInput::MoveDown);

    // Compute horizontal and vertical components
    let h = (right as i8) - (left as i8); // +1, -1, or 0
    let v = (down as i8) - (up as i8);    // +1, -1, or 0

    // If no direction held, return None
    if h == 0 && v == 0 {
        return None;
    }

    // Map to DIRECTIONS index:
    // DIRECTIONS[0]=(0,-1)=N, [1]=(1,-1)=NE, [2]=(1,0)=E, [3]=(1,1)=SE,
    // [4]=(0,1)=S, [5]=(-1,1)=SW, [6]=(-1,0)=W, [7]=(-1,-1)=NW
    match (h, v) {
        (0, -1) => Some(0),  // N
        (1, -1) => Some(1),  // NE
        (1, 0) => Some(2),   // E
        (1, 1) => Some(3),   // SE
        (0, 1) => Some(4),   // S
        (-1, 1) => Some(5), // SW
        (-1, 0) => Some(6),  // W
        (-1, -1) => Some(7), // NW
        _ => None,
    }
}

/// Player movement system.
/// Reads `Players` resource (virtual gamepad state), applies grid-snapped movement,
/// and writes updated `GridPos` and `PlayerComponent` state.
///
/// Runs in `PostUpdate` after input has been processed in `Update`.
pub fn player_movement_system(
    mut player_query: Query<(Entity, &mut PlayerComponent, &mut GridPos)>,
    players: Res<Players>,
    mut world: ResMut<crate::bevy_plugins::world::VoxpartyWorld>,
) {
    // Collect desired directions for each player entity (immutable pass)
    let mut player_dirs: Vec<(Entity, Option<usize>)> = Vec::new();

    for (entity, player_comp, _grid_pos) in &mut player_query {
        let vg = if player_comp.player_id == 1 {
            &players.player1
        } else {
            &players.player2
        };

        // First try held directional inputs, fall back to analog joystick
        let dir = held_to_direction(vg)
            .or_else(|| quantize_direction(vg.joystick_x, vg.joystick_y));

        player_dirs.push((entity, dir));
    }

    // Process each player's movement (mutable pass)
    for (entity, want_dir) in player_dirs {
        let Ok((_entity, mut player, mut grid_pos)) = player_query.get_mut(entity) else {
            continue;
        };

        // Skip eliminated/won players
        if player.state == PlayerState::Eliminated || player.state == PlayerState::Won {
            continue;
        }

        // Countdown cooldown
        if player.cooldown > 0.0 {
            player.cooldown -= 1.0 / 60.0; // Approximate dt
        }

        // Don't move if in cooldown
        if player.cooldown > 0.0 {
            continue;
        }

        if let Some(dir) = want_dir {
            // Try to move in the desired direction
            if let Some((new_x, new_y)) = try_move(player.grid_x, player.grid_y, dir, &world.world) {
                player.grid_x = new_x;
                player.grid_y = new_y;
                player.facing_dir = dir;
                player.cooldown = 0.15; // 150ms between grid moves
                player.state = PlayerState::Walking;

                // Update GridPos to match
                grid_pos.x = new_x;
                grid_pos.y = new_y;

                // Check traps
                if world.world.check_trap(new_x, new_y) {
                    player.state = PlayerState::Eliminated;
                    continue;
                }

                // Check checkpoint
                if world.world.check_checkpoint(new_x, new_y) {
                    player.checkpoint_x = new_x;
                    player.checkpoint_y = new_y;
                }

                // Check win condition
                if world.world.check_goal(new_x, new_y) {
                    player.state = PlayerState::Won;
                }
            } else {
                // Blocked — go idle
                if player.state == PlayerState::Walking {
                    player.state = PlayerState::Idle;
                }
            }
        } else {
            // No directional input — idle if was walking
            if player.state == PlayerState::Walking {
                player.state = PlayerState::Idle;
            }
        }
    }
}

/// Player animation system.
/// Advances animation timers based on movement state.
/// Sprite frame selection is handled by the sprite spawn system via GridPos.
pub fn player_animation_system(
    player_query: Query<&mut PlayerComponent>,
) {
    // Animation is primarily driven by movement in this architecture.
    // The sprite spawn system selects frames based on PlayerComponent state.
    // This system can be extended for frame timing if needed.
    for _ in &player_query {
        // Placeholder for animation timing logic
    }
}

/// Plugin that registers player systems and resources.
pub struct PlayerPlugin;

impl PlayerPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (
            player_movement_system,
            player_animation_system,
        ));
    }
}
