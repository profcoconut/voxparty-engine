#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneState {
    Menu,
    TitleCard,
    Playing,
    GameOver,
}

/// Scene state machine — drives the overall game flow.
pub struct Scene {
    pub state: SceneState,
    /// Countdown timer for title card (seconds)
    pub title_timer: f32,
    /// Countdown timer for game over screen (seconds)
    pub gameover_timer: f32,
    /// Winner player ID (1 or 2), if game over due to win
    pub winner: Option<u8>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            state: SceneState::Menu,
            title_timer: 3.0,
            gameover_timer: 0.0,
            winner: None,
        }
    }

    /// Transition from Menu → TitleCard.
    pub fn start_game(&mut self) {
        self.state = SceneState::TitleCard;
        self.title_timer = 3.0;
        self.winner = None;
    }

    /// Advance scene timers by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        match self.state {
            SceneState::TitleCard => {
                self.title_timer -= dt;
                if self.title_timer <= 0.0 {
                    self.state = SceneState::Playing;
                }
            }
            SceneState::GameOver => {
                self.gameover_timer -= dt;
            }
            _ => {}
        }
    }

    /// Trigger game over with a winner (or None for draw/timeout).
    pub fn trigger_gameover(&mut self, winner: Option<u8>) {
        self.state = SceneState::GameOver;
        self.winner = winner;
        self.gameover_timer = 5.0; // show game over for 5 seconds
    }

    /// Whether the game over screen timer has elapsed.
    pub fn gameover_done(&self) -> bool {
        self.state == SceneState::GameOver && self.gameover_timer <= 0.0
    }

    /// Return to the main menu.
    pub fn return_to_menu(&mut self) {
        self.state = SceneState::Menu;
    }
}

impl Default for Scene {
    fn default() -> Self { Self::new() }
}
