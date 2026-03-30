//! Scene state management plugin for VoxParty.
//!
//! Phase 6: Migrates from manual SceneState enum + state machine to Bevy State API.
//!
//! This plugin provides:
//! - `SceneData` resource: holds per-state data (timers, flags, save_data)
//! - Timer resources: TitleCardTimer, GameOverTimer, VictoryTimer, TitlecardFadeTimer, TitlecardPulseTimer
//! - Timer tick systems: automatically advance timers and trigger state transitions
//! - `GameStateSignal`: shared state that both the SDL2 loop and Bevy systems can access
//! - Transition functions: called from lib.rs to signal state changes to Bevy
//!
//! The Bevy `GameState` enum (from `game_state.rs`) drives which systems run.

use bevy::prelude::*;
use std::sync::{Arc, Mutex};

// Re-export SceneData for use by other modules
pub use crate::core::scene::SceneData;
pub use crate::core::scene::SceneState;
use super::game_state::GameState;

// ─── Shared Game State Signal ─────────────────────────────────────────────────
//
// GameStateSignal bridges the SDL2 game loop (lib.rs) and Bevy systems.
// The SDL2 loop calls transition functions here; Bevy systems read from this signal
// and call NextState<GameState> to actually drive the state machine.
//
// In the current SDL2-only architecture, lib.rs uses SceneData.state directly.
// The transition functions below update both SceneData.state (SDL2) and GameStateSignal
// (Bevy). When lib.rs migrates to use Bevy state, only the transition functions
// need updating.

/// Shared game state signal — bridges SDL2 loop and Bevy State.
///
/// Read/Write from both SDL2 loop (lib.rs) and Bevy systems.
/// Both contexts write to this signal; Bevy also calls NextState<GameState>.
#[derive(Resource, Clone)]
pub struct GameStateSignal {
    /// The canonical game state (matches Bevy GameState).
    pub state: Arc<Mutex<GameState>>,
}

impl GameStateSignal {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(GameState::Menu)),
        }
    }

    /// Get the current game state.
    pub fn get(&self) -> GameState {
        self.state.lock().unwrap().clone()
    }

    /// Set the game state. Called by transition functions.
    pub fn set(&self, new_state: GameState) {
        *self.state.lock().unwrap() = new_state;
    }
}

impl Default for GameStateSignal {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Transition Functions ─────────────────────────────────────────────────────
//
// These functions update both SceneData.state (for SDL2 loop) and GameStateSignal
// (for Bevy). They are called from lib.rs keyboard/gamepad input handlers.
//
// Unit 4: These replace scene.start_episode_select(), scene.start_game(),
// scene.return_to_menu(), scene.toggle_pause(), etc.

/// Transition: Menu → EpisodeSelect.
/// Called from lib.rs keyboard/gamepad input.
pub fn go_to_episode_select(scene: &mut SceneData, signal: &GameStateSignal) {
    scene.state = SceneState::EpisodeSelect;
    scene.selected_episode_index = 0;
    signal.set(GameState::EpisodeSelect);
}

/// Transition: EpisodeSelect → TitleCard (start game).
/// Called from lib.rs after reloading episode state.
pub fn go_to_titlecard(scene: &mut SceneData, signal: &GameStateSignal) {
    scene.state = SceneState::TitleCard;
    scene.title_timer = 3.0;
    scene.winner = None;
    scene.time_elapsed = 0.0;
    scene.titlecard_fade_timer = 1.0;
    scene.titlecard_pulse_timer = 0.0;
    if !scene.save_data.tutorial_shown {
        scene.tutorial_visible = true;
    }
    scene.player_died_this_run = false;
    scene.checkpoint_hit_this_run = false;
    scene.npc_seen_this_episode = false;
    scene.death_recap = None;
    scene.death_recap_timer = 0.0;
    signal.set(GameState::TitleCard);
}

/// Transition: Playing/Paused/Victory/GameOver → Menu.
/// Called from lib.rs keyboard/gamepad input or timer expiry.
pub fn go_to_menu(scene: &mut SceneData, signal: &GameStateSignal) {
    scene.state = SceneState::Menu;
    scene.winner = None;
    signal.set(GameState::Menu);
}

/// Transition: Playing ↔ Paused (toggle).
/// Called from lib.rs ESC key or gamepad Start button.
pub fn toggle_pause(scene: &mut SceneData, signal: &GameStateSignal) {
    match scene.state {
        SceneState::Playing => {
            scene.state = SceneState::Paused;
            signal.set(GameState::Paused);
        }
        SceneState::Paused => {
            scene.state = SceneState::Playing;
            signal.set(GameState::Playing);
        }
        _ => {}
    }
}

/// Transition: Playing → Victory.
/// Called from lib.rs when player reaches goal.
pub fn trigger_victory(scene: &mut SceneData, signal: &GameStateSignal, winner: Option<u8>) {
    scene.state = SceneState::Victory;
    scene.winner = winner;
    scene.victory_timer = 10.0;
    signal.set(GameState::Victory);
}

/// Transition: Playing → GameOver.
/// Called from lib.rs when both players eliminated.
pub fn trigger_gameover(scene: &mut SceneData, signal: &GameStateSignal, winner: Option<u8>) {
    scene.state = SceneState::GameOver;
    scene.winner = winner;
    scene.gameover_timer = 5.0;
    signal.set(GameState::GameOver);
}

/// Reload episode game state (reset players, world, NPCs, camera).
/// Called before go_to_titlecard() when starting a new episode.
pub fn reload_episode_state(
    scene: &mut SceneData,
    episode_path: &str,
    screen_w: u32,
    screen_h: u32,
) -> Result<(crate::game::Episode, crate::game::World, crate::game::Player, crate::game::Player, Vec<crate::game::Npc>, crate::core::Camera), String> {
    let episode = crate::game::Episode::load(episode_path)
        .map_err(|e| format!("Failed to load episode: {}", e))?;
    let world = crate::game::World::from_episode(episode.clone());

    let spawn1 = episode.spawn_points.iter().find(|s| s.player == 1)
        .ok_or_else(|| format!("Player 1 spawn point missing"))?;
    let spawn2 = episode.spawn_points.iter().find(|s| s.player == 2)
        .ok_or_else(|| format!("Player 2 spawn point missing"))?;
    let player1 = crate::game::Player::new(1, spawn1.x, spawn1.y);
    let player2 = crate::game::Player::new(2, spawn2.x, spawn2.y);
    let npcs: Vec<crate::game::Npc> = episode
        .npcs
        .iter()
        .map(|n| crate::game::Npc::new(n.x, n.y, n.name.clone(), n.dialogue.clone()))
        .collect();
    let camera = crate::core::Camera::new(screen_w, screen_h, episode.grid_width, episode.grid_height);

    scene.set_last_episode(&episode.id);
    scene.player_died_this_run = false;
    scene.checkpoint_hit_this_run = false;
    scene.npc_seen_this_episode = false;
    scene.death_recap = None;
    scene.death_recap_timer = 0.0;

    Ok((episode, world, player1, player2, npcs, camera))
}

// ─── Timer Resources ─────────────────────────────────────────────────────────

/// Timer for the TitleCard state (3.0s auto-transition to Playing)
#[derive(Resource)]
pub struct TitleCardTimer(pub Timer);

impl TitleCardTimer {
    pub fn new() -> Self {
        Self(Timer::from_seconds(3.0, TimerMode::Once))
    }
}

impl Default for TitleCardTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer for the GameOver state (5.0s auto-transition to Menu)
#[derive(Resource)]
pub struct GameOverTimer(pub Timer);

impl GameOverTimer {
    pub fn new() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Once))
    }
}

impl Default for GameOverTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer for the Victory state (10.0s auto-transition to Menu)
#[derive(Resource)]
pub struct VictoryTimer(pub Timer);

impl VictoryTimer {
    pub fn new() -> Self {
        Self(Timer::from_seconds(10.0, TimerMode::Once))
    }
}

impl Default for VictoryTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Fade-in timer for episode name on title card (1.0s)
#[derive(Resource)]
pub struct TitlecardFadeTimer(pub Timer);

impl TitlecardFadeTimer {
    pub fn new() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Once))
    }
}

impl Default for TitlecardFadeTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Pulse accumulator for "GET READY..." text (wraps at 0.8s)
#[derive(Resource)]
pub struct TitlecardPulseTimer(pub Timer);

impl TitlecardPulseTimer {
    pub fn new() -> Self {
        // Using Repeating mode with duration of 0.8s for wrapping behavior
        Self(Timer::from_seconds(0.8, TimerMode::Repeating))
    }
}

impl Default for TitlecardPulseTimer {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Timer Initialization Systems ────────────────────────────────────────────

/// System that runs when entering TitleCard state to initialize all title card timers.
pub fn init_titlecard_timers(
    mut titlecard_timer: ResMut<TitleCardTimer>,
    mut fade_timer: ResMut<TitlecardFadeTimer>,
    mut pulse_timer: ResMut<TitlecardPulseTimer>,
) {
    titlecard_timer.0 = Timer::from_seconds(3.0, TimerMode::Once);
    fade_timer.0 = Timer::from_seconds(1.0, TimerMode::Once);
    pulse_timer.0 = Timer::from_seconds(0.8, TimerMode::Repeating);
}

/// System that runs when entering GameOver state to initialize the game over timer.
pub fn init_gameover_timers(mut gameover_timer: ResMut<GameOverTimer>) {
    gameover_timer.0 = Timer::from_seconds(5.0, TimerMode::Once);
}

/// System that runs when entering Victory state to initialize the victory timer.
pub fn init_victory_timers(mut victory_timer: ResMut<VictoryTimer>) {
    victory_timer.0 = Timer::from_seconds(10.0, TimerMode::Once);
}

/// System that runs when entering Playing state to reset per-run timers and flags.
pub fn init_playing_state(
    mut scene_data: ResMut<SceneData>,
) {
    scene_data.time_elapsed = 0.0;
    scene_data.player_died_this_run = false;
    scene_data.checkpoint_hit_this_run = false;
    scene_data.npc_seen_this_episode = false;
    scene_data.death_recap = None;
    scene_data.death_recap_timer = 0.0;
}

/// System that runs when entering TitleCard state to reset game state for a new episode.
pub fn init_titlecard_state(
    mut scene_data: ResMut<SceneData>,
) {
    scene_data.winner = None;
    // Request tutorial if not yet shown
    if !scene_data.save_data.tutorial_shown {
        scene_data.tutorial_visible = true;
    }
}

/// System that runs when entering Menu state to reset winner.
pub fn init_menu_state(
    mut scene_data: ResMut<SceneData>,
) {
    scene_data.winner = None;
}

/// System that runs when entering GameOver state.
pub fn init_gameover_state() {
    // Winner is set by game logic systems before transitioning to GameOver state
    // This system is a placeholder for future initialization
}

/// System that runs when entering Victory state.
pub fn init_victory_state() {
    // Winner is set by game logic systems before transitioning to Victory state
    // This system is a placeholder for future initialization
}

/// System that runs when entering EpisodeSelect state to reset selection.
pub fn init_episode_select_state(
    mut scene_data: ResMut<SceneData>,
) {
    scene_data.selected_episode_index = 0;
}

// ─── OnExit Systems ───────────────────────────────────────────────────────────
//
// Unit 3: Per-state cleanup. These systems run when leaving each game state.
// Used for hiding UI elements, flushing telemetry, or stopping effects.

/// Cleanup when exiting Menu state.
pub fn cleanup_menu_state() {
    // Menu UI is cleared by SDL2 render match; no entities to despawn here.
}

/// Cleanup when exiting EpisodeSelect state.
pub fn cleanup_episode_select_state() {
    // Episode selection UI is cleared by SDL2 render match.
}

/// Cleanup when exiting TitleCard state.
pub fn cleanup_titlecard_state() {
    // Title card UI is cleared by SDL2 render match.
}

/// Cleanup when exiting Playing state.
pub fn cleanup_playing_state(
    mut scene_data: ResMut<SceneData>,
) {
    // Flush telemetry session when leaving Playing state.
    // Note: telemetry_session_end() is called from lib.rs when returning to Menu.
    // This system provides a Bevy-side hook for future telemetry integration.
}

// ─── Timer Tick Systems ──────────────────────────────────────────────────────

/// System that ticks the TitleCard timer and transitions to Playing when expired.
pub fn tick_titlecard_timer(
    mut titlecard_timer: ResMut<TitleCardTimer>,
    mut next_state: ResMut<NextState<crate::bevy_plugins::game_state::GameState>>,
    time: Res<Time>,
) {
    titlecard_timer.0.tick(time.delta());
    if titlecard_timer.0.just_finished() {
        next_state.set(crate::bevy_plugins::game_state::GameState::Playing);
    }
}

/// System that ticks the GameOver timer and transitions to Menu when expired.
pub fn tick_gameover_timer(
    mut gameover_timer: ResMut<GameOverTimer>,
    mut next_state: ResMut<NextState<crate::bevy_plugins::game_state::GameState>>,
    time: Res<Time>,
) {
    gameover_timer.0.tick(time.delta());
    if gameover_timer.0.just_finished() {
        next_state.set(crate::bevy_plugins::game_state::GameState::Menu);
    }
}

/// System that ticks the Victory timer and transitions to Menu when expired.
pub fn tick_victory_timer(
    mut victory_timer: ResMut<VictoryTimer>,
    mut next_state: ResMut<NextState<crate::bevy_plugins::game_state::GameState>>,
    time: Res<Time>,
) {
    victory_timer.0.tick(time.delta());
    if victory_timer.0.just_finished() {
        next_state.set(crate::bevy_plugins::game_state::GameState::Menu);
    }
}

/// System that ticks the TitlecardFadeTimer.
pub fn tick_titlecard_fade_timer(
    mut fade_timer: ResMut<TitlecardFadeTimer>,
    time: Res<Time>,
) {
    fade_timer.0.tick(time.delta());
}

/// System that ticks the TitlecardPulseTimer.
pub fn tick_titlecard_pulse_timer(
    mut pulse_timer: ResMut<TitlecardPulseTimer>,
    time: Res<Time>,
) {
    pulse_timer.0.tick(time.delta());
}

/// System that increments time_elapsed during Playing state.
pub fn tick_playing_time_elapsed(
    mut scene_data: ResMut<SceneData>,
    time: Res<Time>,
) {
    scene_data.time_elapsed += time.delta().as_secs_f32();
}

/// System that decrements death_recap_timer during Playing state.
pub fn tick_death_recap_timer(
    mut scene_data: ResMut<SceneData>,
    time: Res<Time>,
) {
    if scene_data.death_recap_timer > 0.0 {
        scene_data.death_recap_timer -= time.delta().as_secs_f32();
        if scene_data.death_recap_timer <= 0.0 {
            scene_data.death_recap = None;
            scene_data.death_recap_timer = 0.0;
        }
    }
}

// ─── ScenePlugin ─────────────────────────────────────────────────────────────

/// Plugin for scene state management.
///
/// This plugin:
/// - Registers `SceneData` as a resource
/// - Registers all timer resources
/// - Registers `GameStateSignal` for cross-context state communication
/// - Adds systems that run per-state (OnEnter, OnUpdate)
pub struct ScenePlugin;

impl ScenePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ScenePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        // Register SceneData resource (non-send because SaveData may need interior mutability)
        app.init_resource::<SceneData>();

        // Register GameStateSignal for SDL2 ↔ Bevy state bridge
        app.init_resource::<GameStateSignal>();

        // Register timer resources
        app.init_resource::<TitleCardTimer>();
        app.init_resource::<GameOverTimer>();
        app.init_resource::<VictoryTimer>();
        app.init_resource::<TitlecardFadeTimer>();
        app.init_resource::<TitlecardPulseTimer>();

        // ─── OnEnter systems ────────────────────────────────────────────────
        // Unit 3: Per-state setup for rendering and game logic.

        app.add_systems(OnEnter(crate::bevy_plugins::game_state::GameState::Menu), (
            init_menu_state,
        ));

        app.add_systems(OnEnter(crate::bevy_plugins::game_state::GameState::EpisodeSelect), (
            init_episode_select_state,
        ));

        // reload_episode_state runs BEFORE titlecard_enter_system to set up game entities.
        // The SDL2 loop handles this in practice (calls reload_episode_state then go_to_titlecard).
        app.add_systems(OnEnter(crate::bevy_plugins::game_state::GameState::TitleCard), (
            init_titlecard_timers,
            init_titlecard_state,
        ));

        app.add_systems(OnEnter(crate::bevy_plugins::game_state::GameState::Playing), (
            init_playing_state,
        ));

        app.add_systems(OnEnter(crate::bevy_plugins::game_state::GameState::GameOver), (
            init_gameover_timers,
        ));

        app.add_systems(OnEnter(crate::bevy_plugins::game_state::GameState::Victory), (
            init_victory_timers,
        ));

        // ─── OnExit systems ─────────────────────────────────────────────────
        // Unit 3: Per-state cleanup.

        app.add_systems(OnExit(crate::bevy_plugins::game_state::GameState::Menu), (
            cleanup_menu_state,
        ));

        app.add_systems(OnExit(crate::bevy_plugins::game_state::GameState::EpisodeSelect), (
            cleanup_episode_select_state,
        ));

        app.add_systems(OnExit(crate::bevy_plugins::game_state::GameState::TitleCard), (
            cleanup_titlecard_state,
        ));

        app.add_systems(OnExit(crate::bevy_plugins::game_state::GameState::Playing), (
            cleanup_playing_state,
        ));

        // ─── Update systems (per-state timers) ────────────────────────────
        // Unit 2: Bevy Timer resources replace manual f32 timer fields.

        // TitleCard: tick titlecard and UI timers
        app.add_systems(Update, (
            tick_titlecard_timer.run_if(in_state(crate::bevy_plugins::game_state::GameState::TitleCard)),
            tick_titlecard_fade_timer.run_if(in_state(crate::bevy_plugins::game_state::GameState::TitleCard)),
            tick_titlecard_pulse_timer.run_if(in_state(crate::bevy_plugins::game_state::GameState::TitleCard)),
        ));

        // GameOver: tick gameover timer
        app.add_systems(Update, (
            tick_gameover_timer.run_if(in_state(crate::bevy_plugins::game_state::GameState::GameOver)),
        ));

        // Victory: tick victory timer
        app.add_systems(Update, (
            tick_victory_timer.run_if(in_state(crate::bevy_plugins::game_state::GameState::Victory)),
        ));

        // Playing: tick time elapsed and death recap
        app.add_systems(Update, (
            tick_playing_time_elapsed.run_if(in_state(crate::bevy_plugins::game_state::GameState::Playing)),
            tick_death_recap_timer.run_if(in_state(crate::bevy_plugins::game_state::GameState::Playing)),
        ));
    }
}
