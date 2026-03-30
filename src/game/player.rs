use super::world::World;
use crate::core::quantize_direction;
use crate::core::sprites::{AnimPlayer, SpriteSheet};
use crate::core::try_move;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Idle,
    Moving,
    Eliminated,
    Won,
}

/// Events that can occur during a player tick, returned for audio/UI wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEvent {
    Moved,
    Eliminated,
    Checkpoint,
    Won,
}

pub struct Player {
    pub id: u8,
    pub grid_x: i32,
    pub grid_y: i32,
    /// Current facing direction index (0-7)
    pub facing_dir: usize,
    pub state: PlayerState,
    /// Last checkpoint position
    pub checkpoint_x: i32,
    pub checkpoint_y: i32,
    pub anim: AnimPlayer,
    /// Movement cooldown timer in seconds
    move_cooldown: f32,
    /// Number of times this player has respawned
    pub respawn_count: u32,
}

impl Player {
    pub fn new(id: u8, x: i32, y: i32) -> Self {
        Self {
            id,
            grid_x: x,
            grid_y: y,
            facing_dir: 0,
            state: PlayerState::Idle,
            checkpoint_x: x,
            checkpoint_y: y,
            anim: AnimPlayer::new(),
            move_cooldown: 0.0,
            respawn_count: 0,
        }
    }

    /// Process one frame of player update.
    /// `jx`, `jy` — raw joystick axes (-1..1), deadzone handled internally by quantize_direction.
    /// `world` — mutable world (for traps, checkpoints).
    /// `sheet` — sprite sheet for animation.
    /// Returns an event if something significant happened (for audio/UI wiring).
    pub fn tick(&mut self, dt: f32, jx: f32, jy: f32, world: &mut World, sheet: &SpriteSheet) -> Option<PlayerEvent> {
        // Cooldown countdown
        if self.move_cooldown > 0.0 {
            self.move_cooldown -= dt;
        }

        match self.state {
            PlayerState::Eliminated | PlayerState::Won => return None,
            _ => {}
        }

        // Animate
        let _ = self.anim.tick(dt, sheet);

        // Don't move if still in cooldown
        if self.move_cooldown > 0.0 {
            return None;
        }

        // Determine 8-way direction directly from joystick using quantize_direction
        let want_dir = quantize_direction(jx, jy);

        if let Some(dir) = want_dir {
            if let Some((new_x, new_y)) = try_move(self.grid_x, self.grid_y, dir, world) {
                self.grid_x = new_x;
                self.grid_y = new_y;
                self.facing_dir = dir;
                self.move_cooldown = 0.15; // seconds between grid moves

                // Check traps (after moving — player stands on trap tile)
                if world.check_trap(new_x, new_y) {
                    self.state = PlayerState::Eliminated;
                    return Some(PlayerEvent::Eliminated);
                }

                // Check checkpoint
                let got_checkpoint = world.check_checkpoint(new_x, new_y);
                if got_checkpoint {
                    self.checkpoint_x = new_x;
                    self.checkpoint_y = new_y;
                }

                // Check win condition
                if world.check_goal(new_x, new_y) {
                    self.state = PlayerState::Won;
                    return Some(PlayerEvent::Won);
                }

                // Still alive and moved — update animation
                let anim_name = if self.id == 1 { "run_p1" } else { "run_p2" };
                self.anim.play(anim_name, sheet, false);
                self.anim.advance(sheet);
                self.state = PlayerState::Moving;
                return Some(if got_checkpoint { PlayerEvent::Checkpoint } else { PlayerEvent::Moved });
            } else {
                // Blocked — go idle
                if self.state == PlayerState::Moving {
                    self.state = PlayerState::Idle;
                    let idle_name = if self.id == 1 { "idle_p1" } else { "idle_p2" };
                    self.anim.play(idle_name, sheet, false);
                }
            }
        } else {
            // No directional input
            if self.state == PlayerState::Moving {
                self.state = PlayerState::Idle;
                let idle_name = if self.id == 1 { "idle_p1" } else { "idle_p2" };
                self.anim.play(idle_name, sheet, false);
            }
        }
        None
    }

    /// Respawn at last checkpoint (call after elimination).
    pub fn respawn(&mut self) {
        self.grid_x = self.checkpoint_x;
        self.grid_y = self.checkpoint_y;
        self.state = PlayerState::Idle;
        self.respawn_count += 1;
    }

    /// Return remaining lives (3 - respawn_count).
    pub fn lives(&self) -> u32 {
        3u32.saturating_sub(self.respawn_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sprites::SpriteSheet;
    use crate::game::episode::Episode;
    use std::collections::HashMap;

    fn make_player() -> Player {
        Player::new(1, 5, 5)
    }

    fn make_world() -> World {
        let json = r#"{
            "id": "ep_test",
            "title": "Test",
            "mode": "solo",
            "theme": "cave",
            "difficulty": "easy",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 10,
            "grid_height": 10,
            "tiles": [],
            "npcs": [],
            "checkpoints": [],
            "spawn_points": [],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        World::from_episode(ep)
    }

    fn make_sprite_sheet() -> SpriteSheet {
        SpriteSheet {
            name: "test".to_string(),
            frames: HashMap::new(),
            animations: HashMap::new(),
            bytes: 0,
        }
    }

    #[test]
    fn test_new_sets_position_and_checkpoint() {
        let p = make_player();
        assert_eq!(p.grid_x, 5);
        assert_eq!(p.grid_y, 5);
        assert_eq!(p.checkpoint_x, 5);
        assert_eq!(p.checkpoint_y, 5);
        assert_eq!(p.state, PlayerState::Idle);
        assert_eq!(p.facing_dir, 0);
        assert_eq!(p.id, 1);
    }

    #[test]
    fn test_respawn_returns_to_checkpoint() {
        let mut p = Player::new(1, 5, 5);
        p.grid_x = 8;
        p.grid_y = 8;
        p.state = PlayerState::Eliminated;
        p.respawn();
        assert_eq!(p.grid_x, 5);
        assert_eq!(p.grid_y, 5);
        assert_eq!(p.state, PlayerState::Idle);
    }

    /// Regression test for qa-1: checkpoint_y must be set to new_y, not new_x.
    /// When player reaches a checkpoint at (6, 5), checkpoint_y should be 5, not 6.
    #[test]
    fn test_checkpoint_y_is_correctly_set_to_new_y() {
        let json = r#"{
            "id": "ep_cp_y_test",
            "title": "Checkpoint Y Test",
            "mode": "solo",
            "theme": "cave",
            "difficulty": "easy",
            "duration_target_seconds": 60,
            "tile_width": 64,
            "tile_height": 32,
            "grid_width": 10,
            "grid_height": 10,
            "tiles": [],
            "npcs": [],
            "checkpoints": [{"x": 6, "y": 5}],
            "spawn_points": [{"player": 1, "x": 5, "y": 5}],
            "win_condition": {"type": "reach_goal"},
            "fail_condition": {"type": "fall_off_map"}
        }"#;
        let ep: Episode = serde_json::from_str(json).unwrap();
        let spawn_x = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().x;
        let spawn_y = ep.spawn_points.iter().find(|s| s.player == 1).unwrap().y;
        let mut world = World::from_episode(ep);

        let mut player = Player::new(1, spawn_x, spawn_y);

        // Move right to checkpoint at (6, 5)
        let sheet = SpriteSheet::from_json(&crate::assets::loader::load_sprite_sheet("characters"));
        player.tick(0.016, 1.0, 0.0, &mut world, &sheet);

        // Verify checkpoint coordinates are correct
        assert_eq!(player.checkpoint_x, 6, "checkpoint_x should be 6");
        assert_eq!(player.checkpoint_y, 5, "checkpoint_y should be 5, not 6 (qa-1 regression)");
    }

    #[test]
    fn test_tick_eliminated_does_nothing() {
        let mut p = Player::new(1, 5, 5);
        p.state = PlayerState::Eliminated;
        let mut world = make_world();
        let sheet = make_sprite_sheet();
        p.tick(0.016, 0.0, 0.0, &mut world, &sheet);
        assert_eq!(p.state, PlayerState::Eliminated);
    }

    #[test]
    fn test_tick_won_does_nothing() {
        let mut p = Player::new(1, 5, 5);
        p.state = PlayerState::Won;
        let mut world = make_world();
        let sheet = make_sprite_sheet();
        p.tick(0.016, 0.0, 0.0, &mut world, &sheet);
        assert_eq!(p.state, PlayerState::Won);
    }

    #[test]
    fn test_tick_cooldown_prevents_movement() {
        let mut p = Player::new(1, 5, 5);
        p.move_cooldown = 0.1;
        let mut world = make_world();
        let sheet = make_sprite_sheet();
        p.tick(0.016, 1.0, 0.0, &mut world, &sheet);
        assert_eq!(p.grid_x, 5, "player should not move while in cooldown");
    }

    #[test]
    fn test_tick_no_inputs_idle_stays_idle() {
        let mut p = Player::new(1, 5, 5);
        p.state = PlayerState::Idle;
        let mut world = make_world();
        let sheet = make_sprite_sheet();
        p.tick(0.016, 0.0, 0.0, &mut world, &sheet);
        assert_eq!(p.state, PlayerState::Idle);
    }

    #[test]
    fn test_player_state_enum_variants() {
        assert_eq!(PlayerState::Idle, PlayerState::Idle);
        assert_eq!(PlayerState::Moving, PlayerState::Moving);
        assert_eq!(PlayerState::Eliminated, PlayerState::Eliminated);
        assert_eq!(PlayerState::Won, PlayerState::Won);
    }

    #[test]
    fn test_new_player_has_zero_cooldown() {
        let p = make_player();
        assert_eq!(p.move_cooldown, 0.0);
    }

    #[test]
    fn test_tick_respects_cooldown_and_countdown() {
        let mut p = Player::new(1, 5, 5);
        p.move_cooldown = 0.1;
        let mut world = make_world();
        let sheet = make_sprite_sheet();
        p.tick(0.016, 0.0, 0.0, &mut world, &sheet);
        // After one tick at dt=0.016, cooldown should be ~0.084
        assert!(p.move_cooldown < 0.1);
        assert!(p.move_cooldown > 0.0);
    }
}
