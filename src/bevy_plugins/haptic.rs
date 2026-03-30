//! Phase 5: Mobile Haptic Feedback as Bevy Plugin.
//!
//! Wraps `crate::platform::haptic::HapticManager` as a Bevy non-send resource and provides
//! a `HapticEvent` enum that systems can trigger to request haptic feedback.
//!
//! ## HapticEvent
//!
//! Systems trigger `HapticEvent` values (Bevy Events via observers) to request haptics:
//! - `HapticEvent::Move` -- 30ms, 0.5 intensity (light tap on movement)
//! - `HapticEvent::Checkpoint` -- 50ms, 0.3 intensity (softer checkpoint vibration)
//! - `HapticEvent::Trap` -- 200ms, 1.0 intensity (heavy elimination vibration)
//! - `HapticEvent::Button` -- 20ms, 0.5 intensity (menu/UI button press)
//!
//! ## haptic_playback_observer
//!
//! An observer that listens for `HapticEvent` triggers and dispatches to `HapticManager`.

use bevy::prelude::{App, Event, NonSendMut, On, Plugin};
use crate::platform::haptic::HapticManager;

/// Wrapper that holds the platform `HapticManager` for use in Bevy.
///
/// `HapticManager` wraps platform haptic state (native object pointers on iOS/Android)
/// which are `!Send`, so this type is stored as a non-send resource and accessed
/// via `NonSendMut<BevyHapticManager>`.
pub struct BevyHapticManager {
    inner: HapticManager,
}

impl BevyHapticManager {
    /// Construct a new `BevyHapticManager`, wrapping the platform `HapticManager`.
    /// Returns an error if haptic subsystem cannot be initialized.
    pub fn new() -> Result<Self, String> {
        let hm = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| HapticManager::new(None)))
            .map_err(|_| "Failed to initialize haptic output".to_string())?;
        Ok(Self { inner: hm })
    }

    /// Trigger a vibration for `millis` duration with `intensity` in [0.0, 1.0].
    pub fn vibrate(&mut self, millis: u32, intensity: f32) {
        self.inner.vibrate(millis, intensity);
    }
}

/// Bevy event enum for haptic feedback requests.
/// Triggered by game systems via `commands.trigger(HapticEvent::...)`;
/// consumed by the `haptic_playback_observer`.
#[derive(Event, Debug, Clone, PartialEq)]
pub enum HapticEvent {
    /// Light tap on movement (30ms, 0.5 intensity).
    Move,
    /// Softer checkpoint vibration (50ms, 0.3 intensity).
    Checkpoint,
    /// Heavy elimination vibration (200ms, 1.0 intensity).
    Trap,
    /// Menu/UI button press (20ms, 0.5 intensity).
    Button,
}

/// Observer that listens for `HapticEvent` triggers and dispatches to `BevyHapticManager`.
pub fn haptic_playback_observer(
    event: On<HapticEvent>,
    mut haptic: NonSendMut<BevyHapticManager>,
) {
    match &*event {
        HapticEvent::Move => haptic.vibrate(30, 0.5),
        HapticEvent::Checkpoint => haptic.vibrate(50, 0.3),
        HapticEvent::Trap => haptic.vibrate(200, 1.0),
        HapticEvent::Button => haptic.vibrate(20, 0.5),
    }
}

/// Bevy plugin that registers the haptic observer and inserts `BevyHapticManager` as a non-send resource.
///
/// Haptics are non-critical -- if the haptic subsystem fails to initialize, the plugin logs
/// a warning and continues without haptic feedback rather than crashing the game.
pub struct HapticPlugin;

impl HapticPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for HapticPlugin {
    fn build(&self, app: &mut App) {
        // Try to construct the BevyHapticManager. Haptics are non-critical,
        // so we handle initialization failure gracefully and continue without haptics.
        match BevyHapticManager::new() {
            Ok(haptic_manager) => {
                app.insert_non_send_resource(haptic_manager);
                app.add_observer(haptic_playback_observer);
            }
            Err(e) => {
                log::warn!(
                    "[Haptics] Failed to initialize haptic feedback (non-critical): {}. \
                     Continuing without haptics.",
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
    fn test_haptic_event_move_constructible() {
        let event = HapticEvent::Move;
        assert!(matches!(event, HapticEvent::Move));
    }

    #[test]
    fn test_haptic_event_trap_constructible() {
        let event = HapticEvent::Trap;
        assert!(matches!(event, HapticEvent::Trap));
    }

    #[test]
    fn test_haptic_event_checkpoint_constructible() {
        let event = HapticEvent::Checkpoint;
        assert!(matches!(event, HapticEvent::Checkpoint));
    }

    #[test]
    fn test_haptic_event_button_constructible() {
        let event = HapticEvent::Button;
        assert!(matches!(event, HapticEvent::Button));
    }

    #[test]
    fn test_haptic_event_clone() {
        let event = HapticEvent::Trap;
        let cloned = event.clone();
        assert!(matches!(cloned, HapticEvent::Trap));
    }

    #[test]
    fn test_haptic_event_debug() {
        let event = HapticEvent::Move;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Move"));
    }

    #[test]
    fn test_haptic_event_distinct_discriminants() {
        // Same discriminant check
        let move_evt = HapticEvent::Move;
        let trap_evt = HapticEvent::Trap;
        assert_ne!(
            std::mem::discriminant(&move_evt),
            std::mem::discriminant(&trap_evt)
        );
    }
}
