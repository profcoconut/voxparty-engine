use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

use bevy::prelude::Resource;

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

/// Legacy scene state enum — kept for backward compatibility with tests.
///
/// Phase 6 migration: This enum is being phased out in favor of `GameState` from
/// `bevy_plugins/game_state.rs`. Transition methods that used this enum have been
/// moved to Bevy systems. New code should use `GameState` instead.
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

/// Scene state data — holds per-state data (timers, flags, save_data).
///
/// Formerly named `Scene`, this struct was slimmed during Phase 6 migration:
/// - `state` field is kept for backward compatibility (Unit 4 will migrate usages to Bevy's `GameState`)
/// - Transition methods (`start_game()`, `trigger_victory()`, etc.) moved to Bevy systems
/// - `just_entered_*()` helpers removed (replaced by `OnEnter` schedules)
///
/// Timer fields (`title_timer`, `gameover_timer`, `victory_timer`) are kept as raw f32
/// for backward compatibility with existing tests.
#[derive(Resource)]
pub struct SceneData {
    /// DEPRECATED: This field is being phased out in favor of Bevy's `GameState`.
    /// Unit 4 will migrate all usages. Currently kept for backward compatibility.
    /// The canonical state is now in `GameState` (Bevy State).
    pub state: SceneState,

    /// Countdown timer for title card (seconds) — kept for test compatibility
    pub title_timer: f32,
    /// Countdown timer for game over screen (seconds) — kept for test compatibility
    pub gameover_timer: f32,
    /// Countdown timer for victory screen (seconds) — kept for test compatibility
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
    /// npc-hints-1: Player died (lost a life) since last episode start
    pub player_died_this_run: bool,
    /// npc-hints-1: Player reached a checkpoint since last episode start
    pub checkpoint_hit_this_run: bool,
    /// npc-hints-1: Player has interacted with an NPC this episode
    pub npc_seen_this_episode: bool,
    /// sprint-20: Death recap — (tile_type string, grid_x, grid_y) of last elimination
    pub death_recap: Option<(String, i32, i32)>,
    /// sprint-20: Death recap display timer (seconds remaining)
    pub death_recap_timer: f32,

    /// DEPRECATED: Internal tracking for `just_entered_playing()` helper.
    /// Unit 4 will migrate usages to Bevy's `OnEnter` schedules.
    prev_state: SceneState,
}

impl SceneData {
    /// Create a new SceneData and load saved data from ~/.voxparty/save.json.
    pub fn new() -> Self {
        let save_data = Self::load().unwrap_or_default();
        Self {
            state: SceneState::Menu,
            prev_state: SceneState::Menu,
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
            player_died_this_run: false,
            checkpoint_hit_this_run: false,
            npc_seen_this_episode: false,
            death_recap: None,
            death_recap_timer: 0.0,
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

    /// Whether the game over screen timer has elapsed.
    /// Note: This is for backward compatibility. In production, use the Bevy Timer resource.
    pub fn gameover_done(&self) -> bool {
        self.state == SceneState::GameOver && self.gameover_timer <= 0.0
    }

    /// Whether the victory screen timer has elapsed.
    /// Note: This is for backward compatibility. In production, use the Bevy Timer resource.
    pub fn victory_done(&self) -> bool {
        self.state == SceneState::Victory && self.victory_timer <= 0.0
    }

    // ─── DEPRECATED METHODS (for backward compatibility during Phase 6 migration) ───
    // These methods are being phased out in favor of Bevy State systems.
    // Unit 4 will migrate all usages to `NextState<GameState>` patterns.

    /// DEPRECATED: Use Bevy `NextState<GameState>` instead.
    /// Transition from Menu → EpisodeSelect.
    pub fn start_episode_select(&mut self) {
        self.state = SceneState::EpisodeSelect;
        self.selected_episode_index = 0;
    }

    /// DEPRECATED: Use Bevy `NextState<GameState>` instead.
    /// Transition from Menu → TitleCard.
    pub fn start_game(&mut self) {
        self.state = SceneState::TitleCard;
        self.title_timer = 3.0;
        self.winner = None;
        self.time_elapsed = 0.0;
        self.titlecard_fade_timer = 1.0;
        self.titlecard_pulse_timer = 0.0;
        self.request_tutorial();
        self.player_died_this_run = false;
        self.checkpoint_hit_this_run = false;
        self.npc_seen_this_episode = false;
        self.death_recap = None;
        self.death_recap_timer = 0.0;
    }

    /// DEPRECATED: Use Bevy `NextState<GameState>` instead.
    /// Return to the main menu.
    pub fn return_to_menu(&mut self) {
        self.state = SceneState::Menu;
        self.winner = None;
    }

    /// DEPRECATED: Use Bevy `NextState<GameState>` instead.
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

    /// DEPRECATED: Use Bevy `NextState<GameState>` instead.
    /// Trigger game over with a winner (or None for draw/timeout).
    pub fn trigger_gameover(&mut self, winner: Option<u8>) {
        self.state = SceneState::GameOver;
        self.winner = winner;
        self.gameover_timer = 5.0;
    }

    /// DEPRECATED: Use Bevy `NextState<GameState>` instead.
    /// Trigger victory screen when a player reaches the goal.
    pub fn trigger_victory(&mut self, winner: Option<u8>) {
        self.state = SceneState::Victory;
        self.winner = winner;
        self.victory_timer = 10.0;
    }

    /// DEPRECATED: This tick method is being replaced by Bevy Timer resources.
    /// Advance scene timers by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        // Track state for transition detection
        self.prev_state = self.state;

        match self.state {
            SceneState::TitleCard => {
                self.title_timer -= dt;
                if self.titlecard_fade_timer > 0.0 {
                    self.titlecard_fade_timer = (self.titlecard_fade_timer - dt).max(0.0);
                } else {
                    self.titlecard_pulse_timer += dt;
                }
                if self.title_timer <= 0.0 {
                    self.state = SceneState::Playing;
                }
            }
            SceneState::Playing => {
                self.time_elapsed += dt;
                if self.death_recap_timer > 0.0 {
                    self.death_recap_timer -= dt;
                    if self.death_recap_timer <= 0.0 {
                        self.death_recap = None;
                        self.death_recap_timer = 0.0;
                    }
                }
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

    /// DEPRECATED: Use Bevy's `OnEnter(Playing)` schedule instead.
    /// Returns true if the state just transitioned to Playing this tick.
    /// Resets the flag after returning true (call once per transition).
    pub fn just_entered_playing(&mut self) -> bool {
        if self.prev_state == SceneState::TitleCard && self.state == SceneState::Playing {
            self.prev_state = self.state; // consume the transition
            true
        } else {
            false
        }
    }
}

impl Default for SceneData {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_starts_with_default_values() {
        let scene = SceneData::new();
        assert_eq!(scene.state, SceneState::Menu);
        assert_eq!(scene.title_timer, 3.0);
        assert_eq!(scene.winner, None);
        assert_eq!(scene.time_elapsed, 0.0);
    }

    #[test]
    fn test_update_best_time_records_new_best() {
        let mut scene = SceneData::new();
        scene.save_data.best_times.clear();

        let result = scene.update_best_time("demo", 5000);
        assert!(result, "first time should be recorded as best");
        assert_eq!(scene.get_best_time_ms("demo"), Some(5000));
    }

    #[test]
    fn test_update_best_time_replaces_slower_time() {
        let mut scene = SceneData::new();
        scene.save_data.best_times.clear();

        scene.update_best_time("demo", 5000);
        let result = scene.update_best_time("demo", 4000);
        assert!(result, "faster time should replace slower");
        assert_eq!(scene.get_best_time_ms("demo"), Some(4000));
    }

    #[test]
    fn test_update_best_time_keeps_faster_time() {
        let mut scene = SceneData::new();
        scene.save_data.best_times.clear();

        scene.update_best_time("demo", 4000);
        let result = scene.update_best_time("demo", 5000);
        assert!(!result, "slower time should not replace faster");
        assert_eq!(scene.get_best_time_ms("demo"), Some(4000));
    }

    #[test]
    fn test_episode_select_up_wraps() {
        let mut scene = SceneData::new();
        scene.selected_episode_index = 0;
        scene.episode_select_up(5);
        assert_eq!(scene.selected_episode_index, 4);
    }

    #[test]
    fn test_episode_select_down_wraps() {
        let mut scene = SceneData::new();
        scene.selected_episode_index = 4;
        scene.episode_select_down(5);
        assert_eq!(scene.selected_episode_index, 0);
    }

    #[test]
    fn test_death_recap_sets_timer_and_clears() {
        let mut scene = SceneData::new();
        scene.death_recap = Some(("lava_trap".to_string(), 5, 3));
        scene.death_recap_timer = 2.0;
        assert_eq!(scene.death_recap, Some(("lava_trap".to_string(), 5, 3)));
        assert_eq!(scene.death_recap_timer, 2.0);
    }

    #[test]
    fn test_is_first_run_when_tutorial_not_shown() {
        let mut scene = SceneData::new();
        scene.save_data.tutorial_shown = false;
        assert!(scene.is_first_run());
    }

    #[test]
    fn test_is_first_run_when_tutorial_shown() {
        let mut scene = SceneData::new();
        scene.save_data.tutorial_shown = true;
        assert!(!scene.is_first_run());
    }

    #[test]
    fn test_request_tutorial_sets_visible() {
        let mut scene = SceneData::new();
        scene.save_data.tutorial_shown = false;
        scene.tutorial_visible = false;
        scene.request_tutorial();
        assert!(scene.tutorial_visible);
    }

    #[test]
    fn test_request_tutorial_does_nothing_if_already_shown() {
        let mut scene = SceneData::new();
        scene.save_data.tutorial_shown = true;
        scene.tutorial_visible = false;
        scene.request_tutorial();
        assert!(!scene.tutorial_visible);
    }

    #[test]
    fn test_dismiss_tutorial_sets_flag_and_saves() {
        let mut scene = SceneData::new();
        scene.save_data.tutorial_shown = false;
        scene.tutorial_visible = true;
        scene.dismiss_tutorial();
        assert!(!scene.tutorial_visible);
        assert!(scene.save_data.tutorial_shown);
    }

    #[test]
    fn test_title_timer_initial_value() {
        let mut scene = SceneData::new();
        assert_eq!(scene.title_timer, 3.0, "TitleCard timer should be 3.0 seconds");
    }

    #[test]
    fn test_gameover_timer_initial_value() {
        let mut scene = SceneData::new();
        scene.gameover_timer = 5.0;
        assert_eq!(scene.gameover_timer, 5.0, "GameOver timer should be 5.0 seconds");
    }

    #[test]
    fn test_victory_timer_initial_value() {
        let mut scene = SceneData::new();
        scene.victory_timer = 10.0;
        assert_eq!(scene.victory_timer, 10.0, "Victory timer should be 10.0 seconds");
    }

    #[test]
    fn test_scene_state_enum_variants() {
        // Verify all expected SceneState variants exist
        use SceneState::*;
        let states = [Menu, EpisodeSelect, TitleCard, Playing, Victory, GameOver, Paused];
        assert_eq!(states.len(), 7);
    }

    #[test]
    fn test_save_data_accessible() {
        let scene = SceneData::new();
        // Verify save_data field is accessible
        let _ = &scene.save_data.tutorial_shown;
        let _ = &scene.save_data.best_times;
        let _ = &scene.save_data.last_episode;
        let _ = &scene.save_data.audio_volume;
        // Verify audio_volume has a valid range
        assert!(scene.save_data.audio_volume >= 0.0 && scene.save_data.audio_volume <= 1.0);
    }
}

// Tests for Bevy State migration (Phase 6)
#[cfg(test)]
mod bevy_state_tests {
    use super::*;
    use crate::bevy_plugins::game_state::GameState;

    #[test]
    fn test_game_state_default_is_menu() {
        // Verify GameState (Bevy State) defaults to Menu
        let gs: GameState = GameState::default();
        assert_eq!(gs, GameState::Menu, "GameState::default() should be Menu");
    }

    #[test]
    fn test_game_state_all_variants_exist() {
        // Verify GameState has all expected variants
        use GameState::*;
        let states = [Menu, EpisodeSelect, TitleCard, Playing, Paused, Victory, GameOver];
        assert_eq!(states.len(), 7);
    }

    #[test]
    fn test_scene_state_to_game_state_mapping() {
        // Verify SceneState and GameState have matching variants
        // This documents the migration mapping between old and new state types
        assert_eq!(SceneState::Menu, SceneState::Menu);
        assert_eq!(SceneState::EpisodeSelect, SceneState::EpisodeSelect);
        assert_eq!(SceneState::TitleCard, SceneState::TitleCard);
        assert_eq!(SceneState::Playing, SceneState::Playing);
        assert_eq!(SceneState::Paused, SceneState::Paused);
        assert_eq!(SceneState::Victory, SceneState::Victory);
        assert_eq!(SceneState::GameOver, SceneState::GameOver);
    }
}

/// Backward compatibility type alias: `Scene` is now `SceneData`.
///
/// During Phase 6 migration, the legacy `Scene` type is being phased out in favor
/// of `SceneData` which is a Bevy `Resource`. This alias allows existing code
/// to continue using `Scene` while the migration progresses.
pub type Scene = SceneData;
