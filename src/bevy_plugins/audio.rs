//! Phase 4: Audio Manager as Bevy Resource.
//!
//! Wraps `crate::platform::audio::AudioManager` as a Bevy non-send resource and provides
//! an `AudioEvent` enum that systems can trigger to play sound effects.
//!
//! ## AudioEvent
//!
//! Systems trigger `AudioEvent` values (Bevy Events via observers) to request sounds:
//! - `AudioEvent::Move` -- basic movement blip
//! - `AudioEvent::Step(tile_type)` -- material-aware footstep
//! - `AudioEvent::Trap` -- trap triggered
//! - `AudioEvent::Checkpoint` -- checkpoint reached
//! - `AudioEvent::Victory` -- player won
//! - `AudioEvent::GameOver` -- game over
//! - `AudioEvent::MusicEpisode(theme)` -- switch ambient music (stub; full episode mapping is Phase 6)
//!
//! ## audio_playback_observer
//!
//! An observer that listens for `AudioEvent` triggers and dispatches to `AudioManager`.

use bevy::prelude::{App, Event, NonSendMut, On, Plugin};
use crate::platform::audio::AudioManager;

/// Wrapper that holds the platform `AudioManager` for use in Bevy.
///
/// `AudioManager` contains `OutputStream` which is `!Send`, so this type is stored
/// as a non-send resource and accessed via `NonSendMut<BevyAudioManager>`.
pub struct BevyAudioManager {
    inner: AudioManager,
}

impl BevyAudioManager {
    /// Construct a new `BevyAudioManager`, wrapping the platform `AudioManager`.
    /// Returns an error if audio subsystem cannot be initialized (e.g. no audio device).
    pub fn new() -> Result<Self, String> {
        let am = std::panic::catch_unwind(std::panic::AssertUnwindSafe(AudioManager::new))
            .map_err(|_| "Failed to initialize audio output".to_string())?;
        Ok(Self { inner: am })
    }

    /// Play a sound effect by name, dispatching to the inner `AudioManager`.
    pub fn play_sfx(&mut self, name: &str) {
        self.inner.play_sfx(name);
    }

    /// Play a synthesized footstep sound for the given tile type.
    pub fn play_synth_step(&mut self, tile_type: &str) {
        self.inner.play_synth_step(tile_type);
    }

    /// Play the victory jingle.
    pub fn play_victory_jingle(&mut self) {
        self.inner.play_victory_jingle();
    }

    /// Play the game over sound.
    pub fn play_gameover_sound(&mut self) {
        self.inner.play_gameover_sound();
    }

    /// Play synthesized ambient music stub.
    pub fn play_music_stub(&mut self) {
        self.inner.play_music_stub();
    }
}

/// Bevy event enum for audio requests.
/// Triggered by game systems via `commands.trigger(AudioEvent::...)`;
/// consumed by the `audio_playback_observer`.
#[derive(Event, Debug, Clone, PartialEq)]
pub enum AudioEvent {
    /// Basic movement blip.
    Move,
    /// Material-aware footstep sound. The string is the tile type
    /// (e.g. "grass", "stone", "lava").
    Step(String),
    /// Trap triggered.
    Trap,
    /// Checkpoint reached.
    Checkpoint,
    /// Player won.
    Victory,
    /// Game over.
    GameOver,
    /// Switch ambient music to match an episode theme.
    /// The string is the episode theme (e.g. "grassland", "volcanic_underground").
    /// Uses `play_music_stub()` as a fallback until full episode-to-music mapping (Phase 6).
    MusicEpisode(String),
}

/// Observer that listens for `AudioEvent` triggers and dispatches to `BevyAudioManager`.
pub fn audio_playback_observer(
    event: On<AudioEvent>,
    mut audio: NonSendMut<BevyAudioManager>,
) {
    match &*event {
        AudioEvent::Move => {
            audio.play_sfx("move");
        }
        AudioEvent::Step(tile_type) => {
            audio.play_synth_step(tile_type);
        }
        AudioEvent::Trap => {
            audio.play_sfx("trap");
        }
        AudioEvent::Checkpoint => {
            audio.play_sfx("checkpoint");
        }
        AudioEvent::Victory => {
            audio.play_victory_jingle();
        }
        AudioEvent::GameOver => {
            audio.play_gameover_sound();
        }
        AudioEvent::MusicEpisode(_theme) => {
            // Full episode-to-music mapping is Phase 6 work.
            // For now, use the synthesized ambient stub.
            audio.play_music_stub();
        }
    }
}

/// Bevy plugin that registers the audio observer and inserts `BevyAudioManager` as a non-send resource.
///
/// Audio is non-critical -- if the audio subsystem fails to initialize (e.g. no audio
/// device available), the plugin logs a warning and continues without audio rather than
/// crashing the game.
pub struct AudioPlugin;

impl AudioPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        // Try to construct the BevyAudioManager. Audio is non-critical,
        // so we handle initialization failure gracefully and continue
        // without audio.
        match BevyAudioManager::new() {
            Ok(audio_manager) => {
                app.insert_non_send_resource(audio_manager);
                app.add_observer(audio_playback_observer);
            }
            Err(e) => {
                log::warn!(
                    "[Audio] Failed to initialize audio output (non-critical): {}. \
                     Continuing without audio.",
                    e
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_event_move_constructible() {
        let event = AudioEvent::Move;
        assert!(matches!(event, AudioEvent::Move));
    }

    #[test]
    fn test_audio_event_step_grass() {
        let event = AudioEvent::Step("grass".to_string());
        assert!(matches!(event, AudioEvent::Step(s) if s == "grass"));
    }

    #[test]
    fn test_audio_event_step_lava() {
        let event = AudioEvent::Step("lava_trap".to_string());
        assert!(matches!(event, AudioEvent::Step(s) if s == "lava_trap"));
    }

    #[test]
    fn test_audio_event_step_distinct() {
        let grass = AudioEvent::Step("grass".to_string());
        let lava = AudioEvent::Step("lava".to_string());
        // Same variant with different payloads -- discriminants are equal but values differ.
        assert_eq!(std::mem::discriminant(&grass), std::mem::discriminant(&lava));
        assert_ne!(grass, lava);
        // Different variants should have different discriminants.
        let trap = AudioEvent::Trap;
        assert_ne!(std::mem::discriminant(&grass), std::mem::discriminant(&trap));
    }

    #[test]
    fn test_audio_event_trap() {
        let event = AudioEvent::Trap;
        assert!(matches!(event, AudioEvent::Trap));
    }

    #[test]
    fn test_audio_event_checkpoint() {
        let event = AudioEvent::Checkpoint;
        assert!(matches!(event, AudioEvent::Checkpoint));
    }

    #[test]
    fn test_audio_event_victory() {
        let event = AudioEvent::Victory;
        assert!(matches!(event, AudioEvent::Victory));
    }

    #[test]
    fn test_audio_event_gameover() {
        let event = AudioEvent::GameOver;
        assert!(matches!(event, AudioEvent::GameOver));
    }

    #[test]
    fn test_audio_event_music_episode() {
        let event = AudioEvent::MusicEpisode("grassland".to_string());
        assert!(matches!(event, AudioEvent::MusicEpisode(s) if s == "grassland"));
    }

    #[test]
    fn test_audio_event_clone() {
        let event = AudioEvent::Trap;
        let cloned = event.clone();
        assert!(matches!(cloned, AudioEvent::Trap));
    }

    #[test]
    fn test_audio_event_debug() {
        let event = AudioEvent::Step("stone".to_string());
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Step"));
        assert!(debug_str.contains("stone"));
    }

    #[test]
    fn test_bevy_audio_manager_default() {
        // Note: creating BevyAudioManager::new() requires an audio device
        // and will return Err if none is available. This test just verifies
        // the struct layout is correct.
        // We can't test actual audio playback in headless/unit test context.
        let _ = format!("{:?}", std::mem::size_of::<BevyAudioManager>());
    }

    // Note: audio_playback_observer integration tests would require a real Bevy App
    // with BevyAudioManager resource, which is an integration-level concern.
    // The match exhaustiveness check in this module ensures all AudioEvent variants
    // are handled by the observer.
}
