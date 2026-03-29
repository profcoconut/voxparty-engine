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
        self.winner = None;
    }
}

impl Default for Scene {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_starts_at_menu() {
        let scene = Scene::new();
        assert_eq!(scene.state, SceneState::Menu);
        assert_eq!(scene.winner, None);
    }

    #[test]
    fn test_start_game_transitions_to_title_card() {
        let mut scene = Scene::new();
        scene.start_game();
        assert_eq!(scene.state, SceneState::TitleCard);
        assert_eq!(scene.title_timer, 3.0);
        assert_eq!(scene.winner, None);
    }

    #[test]
    fn test_tick_title_card_transitions_to_playing() {
        let mut scene = Scene::new();
        scene.start_game();
        assert_eq!(scene.state, SceneState::TitleCard);
        scene.tick(3.0);
        assert_eq!(scene.state, SceneState::Playing, "title timer expired → Playing");
    }

    #[test]
    fn test_tick_title_card_does_not_overflow() {
        let mut scene = Scene::new();
        scene.start_game();
        scene.tick(10.0); // way past expiry
        assert_eq!(scene.state, SceneState::Playing);
    }

    #[test]
    fn test_trigger_gameover() {
        let mut scene = Scene::new();
        scene.trigger_gameover(Some(1));
        assert_eq!(scene.state, SceneState::GameOver);
        assert_eq!(scene.winner, Some(1));
        assert_eq!(scene.gameover_timer, 5.0);
    }

    #[test]
    fn test_gameover_done_false_while_timer_running() {
        let mut scene = Scene::new();
        scene.trigger_gameover(Some(1));
        scene.tick(2.0);
        assert!(!scene.gameover_done(), "gameover not done while timer > 0");
    }

    #[test]
    fn test_gameover_done_true_after_timer_expires() {
        let mut scene = Scene::new();
        scene.trigger_gameover(Some(1));
        scene.tick(5.0);
        assert!(scene.gameover_done(), "gameover done after timer expires");
    }

    #[test]
    fn test_return_to_menu() {
        let mut scene = Scene::new();
        scene.trigger_gameover(Some(1));
        scene.return_to_menu();
        assert_eq!(scene.state, SceneState::Menu);
    }

    #[test]
    fn test_return_to_menu_resets_winner() {
        let mut scene = Scene::new();
        scene.trigger_gameover(Some(1));
        scene.return_to_menu();
        assert_eq!(scene.winner, None);
    }
}
