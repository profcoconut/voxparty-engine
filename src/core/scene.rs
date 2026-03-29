use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Save data persisted to ~/.voxparty/save.json
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SaveData {
    /// Whether the tutorial has been shown
    pub tutorial_shown: bool,
    /// Best times per episode ID (in milliseconds)
    pub best_times: HashMap<String, u64>,
    /// Last played episode ID
    pub last_episode: Option<String>,
    /// Audio volume (0.0 to 1.0)
    pub audio_volume: f32,
}


/// Returns the path to the VoxParty save file (~/.voxparty/save.json).
fn save_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("voxparty").join("save.json")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneState {
    Menu,
    EpisodeSelect,
    TitleCard,
    Playing,
    Victory,
    GameOver,
    Paused,
}

/// Scene state machine — drives the overall game flow.
pub struct Scene {
    pub state: SceneState,
    /// Countdown timer for title card (seconds)
    pub title_timer: f32,
    /// Countdown timer for game over screen (seconds)
    pub gameover_timer: f32,
    /// Countdown timer for victory screen (seconds)
    pub victory_timer: f32,
    /// Winner player ID (1 or 2), if game over due to win
    pub winner: Option<u8>,
    /// Time elapsed since game started (seconds)
    pub time_elapsed: f32,
    /// titlecard-anim-1: fade-in timer for episode name (counts down from 1.0)
    pub titlecard_fade_timer: f32,
    /// titlecard-anim-1: pulse accumulator for "GET READY..." (seconds, wraps at 0.8)
    pub titlecard_pulse_timer: f32,
    /// Persisted save data (loaded on startup)
    pub save_data: SaveData,
    /// Whether the tutorial overlay is currently visible
    pub tutorial_visible: bool,
    /// episode-select-1: Index of the currently selected episode in the episode list
    pub selected_episode_index: usize,
}

impl Scene {
    /// Create a new Scene and load saved data from ~/.voxparty/save.json.
    pub fn new() -> Self {
        let save_data = Self::load().unwrap_or_default();
        Self {
            state: SceneState::Menu,
            title_timer: 3.0,
            gameover_timer: 0.0,
            victory_timer: 0.0,
            winner: None,
            time_elapsed: 0.0,
            titlecard_fade_timer: 1.0,
            titlecard_pulse_timer: 0.0,
            save_data,
            tutorial_visible: false,
            selected_episode_index: 0,
        }
    }

    /// Load SaveData from ~/.voxparty/save.json.
    /// Returns None if the file does not exist or is corrupted.
    pub fn load() -> Option<SaveData> {
        let path = save_path();
        let mut file = fs::File::open(&path).ok()?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Save save_data to ~/.voxparty/save.json.
    /// Creates the ~/.voxparty/ directory if it does not exist.
    pub fn save(&self) {
        let path = save_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.save_data) {
            if let Ok(mut file) = fs::File::create(&path) {
                let _ = file.write_all(json.as_bytes());
            }
        }
    }

    /// Returns true if this is the first run (no save data or tutorial not shown yet).
    pub fn is_first_run(&self) -> bool {
        !self.save_data.tutorial_shown
    }

    /// Update the best time for an episode if the given time is faster.
    /// Returns true if a new best was recorded.
    pub fn update_best_time(&mut self, episode_id: &str, time_ms: u64) -> bool {
        let is_new_best = self
            .save_data
            .best_times
            .get(episode_id)
            .map(|&existing| time_ms < existing)
            .unwrap_or(true);
        if is_new_best {
            self.save_data.best_times.insert(episode_id.to_string(), time_ms);
        }
        is_new_best
    }

    /// best-time-1: Get the best time for an episode in milliseconds, if recorded.
    pub fn get_best_time_ms(&self, episode_id: &str) -> Option<u64> {
        self.save_data.best_times.get(episode_id).copied()
    }

    /// Record the last played episode.
    pub fn set_last_episode(&mut self, episode_id: &str) {
        self.save_data.last_episode = Some(episode_id.to_string());
    }

    /// Trigger tutorial to show on next Playing state entry (if not yet shown).
    pub fn request_tutorial(&mut self) {
        if !self.save_data.tutorial_shown {
            self.tutorial_visible = true;
        }
    }

    /// Dismiss the tutorial (call when any key is pressed during tutorial).
    pub fn dismiss_tutorial(&mut self) {
        self.tutorial_visible = false;
        self.save_data.tutorial_shown = true;
        self.save();
    }

    /// episode-select-1: Transition from Menu → EpisodeSelect.
    pub fn start_episode_select(&mut self) {
        self.state = SceneState::EpisodeSelect;
        self.selected_episode_index = 0;
    }

    /// episode-select-1: Move selection up (wrap around).
    pub fn episode_select_up(&mut self, count: usize) {
        if count == 0 { return; }
        self.selected_episode_index = (self.selected_episode_index + count - 1) % count;
    }

    /// episode-select-1: Move selection down (wrap around).
    pub fn episode_select_down(&mut self, count: usize) {
        if count == 0 { return; }
        self.selected_episode_index = (self.selected_episode_index + 1) % count;
    }

    /// Transition from Menu → TitleCard.
    pub fn start_game(&mut self) {
        self.state = SceneState::TitleCard;
        self.title_timer = 3.0;
        self.winner = None;
        self.time_elapsed = 0.0;
        // titlecard-anim-1: reset fade and pulse timers
        self.titlecard_fade_timer = 1.0;
        self.titlecard_pulse_timer = 0.0;
        // tutorial-1: request tutorial overlay before entering Playing
        self.request_tutorial();
    }

    /// Advance scene timers by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        match self.state {
            SceneState::TitleCard => {
                self.title_timer -= dt;
                // titlecard-anim-1: advance fade-in timer
                if self.titlecard_fade_timer > 0.0 {
                    self.titlecard_fade_timer = (self.titlecard_fade_timer - dt).max(0.0);
                } else {
                    // After fade-in completes, advance pulse timer
                    self.titlecard_pulse_timer += dt;
                }
                if self.title_timer <= 0.0 {
                    self.state = SceneState::Playing;
                }
            }
            SceneState::Playing => {
                self.time_elapsed += dt;
            }
            SceneState::Victory => {
                self.victory_timer -= dt;
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

    /// Trigger victory screen when a player reaches the goal.
    pub fn trigger_victory(&mut self, winner: Option<u8>) {
        self.state = SceneState::Victory;
        self.winner = winner;
        self.victory_timer = 10.0; // show victory for 10 seconds
    }

    /// Whether the game over screen timer has elapsed.
    pub fn gameover_done(&self) -> bool {
        self.state == SceneState::GameOver && self.gameover_timer <= 0.0
    }

    /// Whether the victory screen timer has elapsed.
    pub fn victory_done(&self) -> bool {
        self.state == SceneState::Victory && self.victory_timer <= 0.0
    }

    /// Return to the main menu.
    pub fn return_to_menu(&mut self) {
        self.state = SceneState::Menu;
        self.winner = None;
    }

    /// Toggle pause (Playing ↔ Paused). Only meaningful during Playing.
    pub fn toggle_pause(&mut self) {
        match self.state {
            SceneState::Playing => {
                self.state = SceneState::Paused;
            }
            SceneState::Paused => {
                self.state = SceneState::Playing;
            }
            _ => {}
        }
    }

    /// Return true if the game is currently paused.
    pub fn is_paused(&self) -> bool {
        self.state == SceneState::Paused
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

    #[test]
    fn test_trigger_victory() {
        let mut scene = Scene::new();
        scene.trigger_victory(Some(1));
        assert_eq!(scene.state, SceneState::Victory);
        assert_eq!(scene.winner, Some(1));
        assert_eq!(scene.victory_timer, 10.0);
    }

    #[test]
    fn test_victory_done_false_while_timer_running() {
        let mut scene = Scene::new();
        scene.trigger_victory(Some(1));
        scene.tick(5.0);
        assert!(!scene.victory_done(), "victory not done while timer > 0");
    }

    #[test]
    fn test_victory_done_true_after_timer_expires() {
        let mut scene = Scene::new();
        scene.trigger_victory(Some(1));
        scene.tick(10.0);
        assert!(scene.victory_done(), "victory done after timer expires");
    }

    #[test]
    fn test_time_elapsed_increments_during_playing() {
        let mut scene = Scene::new();
        scene.start_game();
        scene.tick(3.0); // transition to Playing
        scene.tick(1.5);
        assert_eq!(scene.time_elapsed, 1.5, "time_elapsed should accumulate during Playing");
    }

    #[test]
    fn test_start_game_resets_time_elapsed() {
        let mut scene = Scene::new();
        scene.start_game();
        scene.tick(3.0); // transition to Playing
        scene.tick(5.0);
        assert_eq!(scene.time_elapsed, 5.0);
        // Start new game
        scene.start_game();
        assert_eq!(scene.time_elapsed, 0.0, "time_elapsed should reset on new game");
    }
}
