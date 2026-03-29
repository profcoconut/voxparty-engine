use super::world::World;
use crate::core::sprites::{AnimPlayer, SpriteSheet};
use crate::core::try_move;
use crate::platform::GameInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Idle,
    Moving,
    Eliminated,
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
        }
    }

    /// Process one frame of player update.
    /// `inputs` — parsed directional inputs from virtual gamepad.
    /// `world` — mutable world (for traps, checkpoints).
    /// `sheet` — sprite sheet for animation.
    pub fn tick(&mut self, dt: f32, inputs: &[GameInput], world: &mut World, sheet: &SpriteSheet) {
        // Cooldown countdown
        if self.move_cooldown > 0.0 {
            self.move_cooldown -= dt;
        }

        match self.state {
            PlayerState::Eliminated | PlayerState::Won => return,
            _ => {}
        }

        // Animate
        let _ = self.anim.tick(dt, sheet);

        // Don't move if still in cooldown
        if self.move_cooldown > 0.0 {
            return;
        }

        // Determine direction from inputs
        let mut want_dir: Option<usize> = None;
        for inp in inputs {
            match inp {
                GameInput::MoveLeft  => want_dir = Some(6), // W
                GameInput::MoveRight => want_dir = Some(2), // E
                GameInput::MoveUp    => want_dir = Some(0), // N
                GameInput::MoveDown  => want_dir = Some(4), // S
                _ => {}
            }
        }

        if let Some(dir) = want_dir {
            if let Some((new_x, new_y)) = try_move(self.grid_x, self.grid_y, dir, world) {
                self.grid_x = new_x;
                self.grid_y = new_y;
                self.facing_dir = dir;
                self.move_cooldown = 0.15; // seconds between grid moves

                // Check traps (after moving — player stands on trap tile)
                if world.check_trap(new_x, new_y) {
                    self.state = PlayerState::Eliminated;
                    return;
                }

                // Check checkpoint
                if world.check_checkpoint(new_x, new_y) {
                    self.checkpoint_x = new_x;
                    self.checkpoint_y = new_y;
                }

                // Check win condition
                if world.check_goal(new_x, new_y) {
                    self.state = PlayerState::Won;
                    return;
                }

                // Still alive and moved — update animation
                let anim_name = if self.id == 1 { "run_p1" } else { "run_p2" };
                self.anim.play(anim_name, sheet, false);
                self.state = PlayerState::Moving;
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
    }

    /// Respawn at last checkpoint (call after elimination).
    pub fn respawn(&mut self) {
        self.grid_x = self.checkpoint_x;
        self.grid_y = self.checkpoint_y;
        self.state = PlayerState::Idle;
    }
}
