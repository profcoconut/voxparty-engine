//! Bevy `State` enum for VoxParty scene management.
//!
//! Replaces the current `SceneState` enum + manual match dispatch in `lib.rs`.
//! Each variant corresponds to a game screen/phase.

use bevy::prelude::*;

/// The primary game state machine — replaces `SceneState` enum.
/// Drives which Bevy schedules run (Menu, Playing, Paused, etc.).
#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub enum GameState {
    /// Main menu screen
    #[default]
    Menu,
    /// Episode selection screen
    EpisodeSelect,
    /// Title card / intro cinematic before an episode
    TitleCard,
    /// Active gameplay
    Playing,
    /// Game is paused (ESC or pause button)
    Paused,
    /// Victory — player(s) reached the goal
    Victory,
    /// Game Over — all players eliminated
    GameOver,
}
