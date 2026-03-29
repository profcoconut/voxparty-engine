use super::episode::Episode;
use crate::core::depth_key;

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
