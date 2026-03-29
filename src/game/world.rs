use super::episode::Episode;
use crate::core::depth_key;
use crate::core::collision::WorldAccess;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Passable,
    Solid,
    Trap,
    Checkpoint,
    Goal,
}

pub struct World {
    pub grid_w: i32,
    pub grid_h: i32,
    /// Flat array: tile_types[y * grid_w + x] = TileType
    tile_types: Vec<TileType>,
    /// Trap cooldown in frames (0 = ready)
    trap_cooldowns: Vec<u32>,
    /// (x, y, reached)
    pub checkpoints: Vec<(i32, i32, bool)>,
    pub episode: Episode,
}

impl World {
    pub fn from_episode(episode: Episode) -> Self {
        let grid_size = (episode.grid_width * episode.grid_height) as usize;
        let mut tile_types = vec![TileType::Passable; grid_size];
        let trap_cooldowns = vec![0u32; grid_size];
        let mut checkpoints = Vec::new();

        for tile in &episode.tiles {
            if tile.x < 0 || tile.y < 0 || tile.x >= episode.grid_width || tile.y >= episode.grid_height {
                continue;
            }
            let idx = (tile.y * episode.grid_width + tile.x) as usize;
            tile_types[idx] = match tile.tile_type.as_str() {
                t if t.ends_with("_solid") => TileType::Solid,
                t if t.ends_with("_trap") => TileType::Trap,
                t if t.ends_with("_checkpoint") => TileType::Checkpoint,
                t if t.ends_with("_goal") => TileType::Goal,
                _ => TileType::Passable,
            };
        }

        for cp in &episode.checkpoints {
            checkpoints.push((cp.x, cp.y, false));
        }

        Self {
            grid_w: episode.grid_width,
            grid_h: episode.grid_height,
            tile_types,
            trap_cooldowns,
            checkpoints,
            episode,
        }
    }

    pub fn get_tile(&self, x: i32, y: i32) -> TileType {
        if x < 0 || y < 0 || x >= self.grid_w || y >= self.grid_h {
            return TileType::Passable; // out of bounds = fall off
        }
        let idx = (y * self.grid_w + x) as usize;
        self.tile_types.get(idx).copied().unwrap_or(TileType::Passable)
    }

    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        self.get_tile(x, y) == TileType::Solid
    }

    /// Check and trigger a trap at (x, y). Returns true if triggered.
    pub fn check_trap(&mut self, x: i32, y: i32) -> bool {
        if self.get_tile(x, y) != TileType::Trap {
            return false;
        }
        let idx = (y * self.grid_w + x) as usize;
        if self.trap_cooldowns[idx] == 0 {
            self.trap_cooldowns[idx] = 180; // 3 seconds at 60fps
            return true;
        }
        false
    }

    /// Decrement trap cooldowns. Call once per frame.
    pub fn tick_trap_cooldowns(&mut self) {
        for c in &mut self.trap_cooldowns {
            if *c > 0 {
                *c -= 1;
            }
        }
    }

    /// Check and activate a checkpoint at (x, y). Returns true if newly reached.
    pub fn check_checkpoint(&mut self, x: i32, y: i32) -> bool {
        for cp in &mut self.checkpoints {
            if cp.0 == x && cp.1 == y && !cp.2 {
                cp.2 = true;
                return true;
            }
        }
        false
    }

    /// Check if (x, y) is a goal tile.
    pub fn check_goal(&self, x: i32, y: i32) -> bool {
        self.get_tile(x, y) == TileType::Goal
    }

    /// Get all non-passable tiles at a given depth, for rendering.
    pub fn tiles_at_depth(&self, depth: i32) -> Vec<(i32, i32, TileType)> {
        let mut result = Vec::new();
        for y in 0..self.grid_h {
            for x in 0..self.grid_w {
                if depth_key(x, y, 0) == depth {
                    let t = self.get_tile(x, y);
                    if t != TileType::Passable {
                        result.push((x, y, t));
                    }
                }
            }
        }
        result
    }
}

impl WorldAccess for World {
    fn grid_w(&self) -> i32 { self.grid_w }
    fn grid_h(&self) -> i32 { self.grid_h }
    fn is_solid(&self, x: i32, y: i32) -> bool { self.is_solid(x, y) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_world_with_tiles() -> World {
        let json = r#"{
            "id": "ep_test",
            "title": "Test",
            "mode": "solo",
            "theme": "cave",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 5,
            "grid_height": 5,
            "tiles": [
                {"x": 1, "y": 1, "type": "stone_solid"},
                {"x": 2, "y": 2, "type": "lava_trap"},
                {"x": 3, "y": 3, "type": "save_checkpoint"},
                {"x": 4, "y": 4, "type": "exit_goal"}
            ],
            "npcs": [],
            "checkpoints": [{"x": 3, "y": 3}],
            "spawn_points": [],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        World::from_episode(ep)
    }

    #[test]
    fn test_get_tile_out_of_bounds_returns_passable() {
        let world = make_world_with_tiles();
        assert_eq!(world.get_tile(-1, 0), TileType::Passable);
        assert_eq!(world.get_tile(0, -1), TileType::Passable);
        assert_eq!(world.get_tile(5, 0), TileType::Passable);
        assert_eq!(world.get_tile(0, 5), TileType::Passable);
    }

    #[test]
    fn test_get_tile_unknown_type_defaults_to_passable() {
        let json = r#"{
            "id": "ep_test2",
            "title": "Test",
            "mode": "solo",
            "theme": "cave",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 3,
            "grid_height": 3,
            "tiles": [{"x": 1, "y": 1, "type": "unknown_type"}],
            "npcs": [],
            "checkpoints": [],
            "spawn_points": [],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        let world = World::from_episode(ep);
        assert_eq!(world.get_tile(1, 1), TileType::Passable);
    }

    #[test]
    fn test_is_solid() {
        let world = make_world_with_tiles();
        assert!(world.is_solid(1, 1));
        assert!(!world.is_solid(0, 0));
        assert!(!world.is_solid(2, 2));
    }

    #[test]
    fn test_check_trap_first_trigger() {
        let mut world = make_world_with_tiles();
        assert!(world.check_trap(2, 2));
    }

    #[test]
    fn test_check_trap_cooldown_blocks_retrigger() {
        let mut world = make_world_with_tiles();
        assert!(world.check_trap(2, 2));
        assert!(!world.check_trap(2, 2), "trap should be on cooldown");
    }

    #[test]
    fn test_check_trap_non_trap_tile_returns_false() {
        let mut world = make_world_with_tiles();
        assert!(!world.check_trap(1, 1));
        assert!(!world.check_trap(0, 0));
    }

    #[test]
    fn test_tick_trap_cooldowns_decrements() {
        let mut world = make_world_with_tiles();
        assert!(world.check_trap(2, 2));
        world.tick_trap_cooldowns();
        // After 1 tick from 180, should be 179 (still blocked)
        assert!(!world.check_trap(2, 2));
    }

    #[test]
    fn test_tick_trap_cooldowns_does_not_below_zero() {
        let json = r#"{
            "id": "ep_test3",
            "title": "Test",
            "mode": "solo",
            "theme": "cave",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 3,
            "grid_height": 3,
            "tiles": [{"x": 1, "y": 1, "type": "lava_trap"}],
            "npcs": [],
            "checkpoints": [],
            "spawn_points": [],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        let mut world = World::from_episode(ep);
        world.tick_trap_cooldowns();
        // Tick should not wrap or go negative
        assert!(world.check_trap(1, 1));
    }

    #[test]
    fn test_check_checkpoint_first_activation() {
        let mut world = make_world_with_tiles();
        assert!(world.check_checkpoint(3, 3));
    }

    #[test]
    fn test_check_checkpoint_already_activated_returns_false() {
        let mut world = make_world_with_tiles();
        world.check_checkpoint(3, 3);
        assert!(!world.check_checkpoint(3, 3));
    }

    #[test]
    fn test_check_checkpoint_non_checkpoint_returns_false() {
        let mut world = make_world_with_tiles();
        assert!(!world.check_checkpoint(1, 1));
        assert!(!world.check_checkpoint(0, 0));
    }

    #[test]
    fn test_check_goal() {
        let world = make_world_with_tiles();
        assert!(world.check_goal(4, 4));
        assert!(!world.check_goal(0, 0));
    }

    #[test]
    fn test_tiles_at_depth_filters_by_depth() {
        let world = make_world_with_tiles();
        // depth_key(1,1,0) = 2, depth_key(2,2,0) = 4, etc.
        let at_depth_2 = world.tiles_at_depth(2);
        assert!(at_depth_2.iter().any(|(x, y, _)| *x == 1 && *y == 1));

        let at_depth_4 = world.tiles_at_depth(4);
        assert!(at_depth_4.iter().any(|(x, y, _)| *x == 2 && *y == 2));

        // Depth with no tiles should return empty
        let empty = world.tiles_at_depth(100);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_tiles_at_depth_excludes_passable() {
        let world = make_world_with_tiles();
        let at_depth_0 = world.tiles_at_depth(0);
        // No non-passable tiles at depth 0 in our test world
        for (x, y, t) in at_depth_0 {
            assert_ne!(t, TileType::Passable, "tile at ({}, {}) should not be Passable", x, y);
        }
    }

    #[test]
    fn test_demo_episode_has_goal_tile() {
        use crate::game::episode::Episode;
        let ep = Episode::load("assets/episodes/demo.json");
        let world = World::from_episode(ep);
        // Demo episode should have at least one goal tile
        let mut found_goal = false;
        for y in 0..world.grid_h {
            for x in 0..world.grid_w {
                if world.check_goal(x, y) {
                    found_goal = true;
                    break;
                }
            }
            if found_goal {
                break;
            }
        }
        assert!(found_goal, "demo episode should have at least one goal tile");
    }

    #[test]
    fn test_demo_episode_has_non_passable_tiles() {
        use crate::game::episode::Episode;
        let ep = Episode::load("assets/episodes/demo.json");
        let world = World::from_episode(ep);
        // Demo episode should have non-passable tiles (walls, traps, etc.)
        let mut found_non_passable = false;
        for y in 0..world.grid_h {
            for x in 0..world.grid_w {
                let tile = world.get_tile(x, y);
                if tile != TileType::Passable {
                    found_non_passable = true;
                    break;
                }
            }
            if found_non_passable {
                break;
            }
        }
        assert!(found_non_passable, "demo episode should have non-passable tiles");
    }
}
