//! World-as-component for VoxParty — episode metadata + world state as Bevy ECS Resource.
//!
//! Provides:
//! - `WorldComponent`: per-episode metadata (episode_id, grid dimensions, elapsed time)
//! - `VoxpartyWorld` Resource: wraps `crate::game::World` with tile query helpers
//! - `world_tick_system`: decrements trap cooldowns, increments elapsed time
//! - `WorldPlugin`: Bevy plugin that inserts VoxpartyWorld resource and registers systems

use bevy::prelude::*;
use crate::game::world::World as GameWorld;
use crate::game::episode::Episode;
use crate::game::TileType;

/// Per-episode metadata component.
/// Stored on a dedicated world-info entity rather than on tile entities.
#[derive(Component, Debug, Clone)]
pub struct WorldComponent {
    pub episode_id: String,
    pub grid_width: i32,
    pub grid_height: i32,
    pub elapsed_time: f32,
}

impl WorldComponent {
    pub fn from_episode(episode: &Episode) -> Self {
        Self {
            episode_id: episode.id.clone(),
            grid_width: episode.grid_width,
            grid_height: episode.grid_height,
            elapsed_time: 0.0,
        }
    }
}

/// World state resource for Bevy systems.
/// Set by the game when starting an episode.
/// Delegates to `crate::game::World` for actual tile/trap/checkpoint logic.
#[derive(Resource, Debug, Clone)]
pub struct VoxpartyWorld {
    pub world: GameWorld,
}

impl VoxpartyWorld {
    pub fn from_episode(episode: Episode) -> Self {
        Self {
            world: GameWorld::from_episode(episode),
        }
    }

    /// Returns the TileType at grid position (x, y).
    pub fn get_tile(&self, x: i32, y: i32) -> TileType {
        self.world.get_tile(x, y)
    }

    /// Returns true if the tile at (x, y) blocks movement.
    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        self.world.is_solid(x, y)
    }

    /// Check if a trap triggers at (x, y). Returns true on first trigger,
    /// false if on cooldown. Resets the trap cooldown on trigger.
    pub fn check_trap(&mut self, x: i32, y: i32) -> bool {
        self.world.check_trap(x, y)
    }

    /// Check if a checkpoint activates at (x, y). Returns true on first activation.
    pub fn check_checkpoint(&mut self, x: i32, y: i32) -> bool {
        self.world.check_checkpoint(x, y)
    }

    /// Returns true if (x, y) is a goal tile.
    pub fn is_goal(&self, x: i32, y: i32) -> bool {
        self.world.check_goal(x, y)
    }
}

/// PostUpdate system: ticks trap cooldowns and increments elapsed time.
pub fn world_tick_system(
    mut world: ResMut<VoxpartyWorld>,
    mut world_comp: Query<&mut WorldComponent>,
) {
    world.world.tick_trap_cooldowns();

    for mut wc in &mut world_comp {
        wc.elapsed_time += 1.0 / 60.0; // Assuming 60fps
    }
}

/// Plugin for world systems.
///
/// Provides:
/// - `VoxpartyWorld` Resource (inserted with a placeholder episode on build)
/// - `WorldComponent` marker (per-episode metadata)
/// - `world_tick_system` (PostUpdate): decrements trap cooldowns, increments elapsed time
pub struct WorldPlugin;

impl WorldPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // Insert a placeholder VoxpartyWorld so systems that require it don't panic.
        // The actual episode world will be loaded when the game starts.
        let placeholder_episode = Episode {
            id: "placeholder".to_string(),
            title: "Placeholder".to_string(),
            mode: "solo".to_string(),
            theme: "grassland".to_string(),
            difficulty: "easy".to_string(),
            duration_target_seconds: 120,
            tile_width: 64,
            tile_height: 32,
            grid_width: 8,
            grid_height: 8,
            tiles: Vec::new(),
            npcs: Vec::new(),
            checkpoints: Vec::new(),
            spawn_points: Vec::new(),
            win_condition: crate::game::episode::WinCondition {
                cond_type: "reach_goal".to_string(),
            },
            fail_condition: crate::game::episode::FailCondition {
                cond_type: "fall_off_map".to_string(),
            },
        };
        let placeholder_world = VoxpartyWorld::from_episode(placeholder_episode);
        app.insert_resource(placeholder_world);

        app.add_systems(PostUpdate, world_tick_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::episode::Episode;

    fn make_episode() -> Episode {
        let json = r#"{
            "id": "ep_test",
            "title": "Test",
            "mode": "solo",
            "theme": "cave",
            "difficulty": "easy",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 5,
            "grid_height": 5,
            "tiles": [
                {"x": 1, "y": 1, "type": "stone_solid"},
                {"x": 2, "y": 2, "type": "lava_trap"},
                {"x": 3, "y": 3, "type": "grass_goal"}
            ],
            "npcs": [],
            "checkpoints": [{"x": 3, "y": 3}],
            "spawn_points": [],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn test_voxparty_world_get_tile() {
        let ep = make_episode();
        let world = VoxpartyWorld::from_episode(ep);
        assert_eq!(world.get_tile(1, 1), TileType::Solid);
        assert_eq!(world.get_tile(2, 2), TileType::Trap);
        assert_eq!(world.get_tile(3, 3), TileType::Goal);
        assert_eq!(world.get_tile(0, 0), TileType::Passable);
    }

    #[test]
    fn test_voxparty_world_is_solid() {
        let ep = make_episode();
        let world = VoxpartyWorld::from_episode(ep);
        assert!(world.is_solid(1, 1));
        assert!(!world.is_solid(0, 0));
        assert!(!world.is_solid(2, 2)); // trap is not solid
    }

    #[test]
    fn test_voxparty_world_check_trap() {
        let ep = make_episode();
        let mut world = VoxpartyWorld::from_episode(ep);
        assert!(world.check_trap(2, 2));
        assert!(!world.check_trap(2, 2), "trap should be on cooldown");
    }

    #[test]
    fn test_voxparty_world_check_checkpoint() {
        let ep = make_episode();
        let mut world = VoxpartyWorld::from_episode(ep);
        assert!(world.check_checkpoint(3, 3));
        assert!(!world.check_checkpoint(3, 3), "checkpoint already activated");
        assert!(!world.check_checkpoint(1, 1));
    }

    #[test]
    fn test_voxparty_world_is_goal() {
        let ep = make_episode();
        let world = VoxpartyWorld::from_episode(ep);
        // No goal tile in test episode
        assert!(!world.is_goal(0, 0));
        assert!(!world.is_goal(1, 1));
    }

    #[test]
    fn test_world_component_from_episode() {
        let ep = make_episode();
        let wc = WorldComponent::from_episode(&ep);
        assert_eq!(wc.episode_id, "ep_test");
        assert_eq!(wc.grid_width, 5);
        assert_eq!(wc.grid_height, 5);
        assert_eq!(wc.elapsed_time, 0.0);
    }
}
