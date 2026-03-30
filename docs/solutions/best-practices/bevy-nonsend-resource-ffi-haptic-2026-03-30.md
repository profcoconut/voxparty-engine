---
title: Mobile Haptics FFI Integration with Bevy Non-Send Resources and Observers
date: 2026-03-30
category: docs/solutions/best-practices
module: bevy_plugins
problem_type: best_practice
component: tooling
severity: medium
applies_when:
  - Integrating platform-specific FFI (Objective-C/Swift Core Haptics, C libraries) into Bevy
  - Wrapping non-Send resources that must only be accessed from the main thread
  - Using Bevy observer pattern for event-driven system communication
tags:
  - bevy
  - mobile
  - haptics
  - ffi
  - nonsend-resource
  - objc2-core-haptics
  - observer
  - ios
related_components:
  - bevy_plugins/haptic.rs
  - platform/haptic.rs
  - bevy_plugins/player.rs
  - bevy_plugins/audio.rs
---

# Mobile Haptics FFI Integration with Bevy Non-Send Resources and Observers

## Context

Phase 5 of the VoxParty Bevy migration required integrating mobile haptics — iOS Core Haptics via the `objc2_core_haptics` FFI crate and an Android no-op stub — into the new Bevy-based game engine. The existing pre-Bevy `HapticManager` was backed by `sdl2::HapticSubsystem`, but Bevy's ECS runs on a separate thread by default, and FFI handles to platform haptic engines are not `Send` (they are bound to the main thread and the Objective-C runtime). Standard Bevy resource injection would fail or cause undefined behavior.

The solution needed to: (1) wrap the FFI-backed haptic manager in a type-safe Bevy non-send resource, (2) expose haptic events through Bevy's observer pattern so game systems could trigger vibration without coupling to platform details, and (3) gracefully degrade when haptics are unavailable (simulators, unsupported devices, Android).

## Guidance

### Non-Send Resource Wrapper Pattern

Wrap the FFI-backed manager in a newtype that does NOT implement `Send`. Use `App::insert_non_send_resource` so Bevy knows this resource must be accessed from the main thread. Wrap initialization in `catch_unwind` to handle cases where the haptic subsystem is unavailable (e.g., desktop, simulators).

```rust
// src/bevy_plugins/haptic.rs

pub struct BevyHapticManager {
    inner: HapticManager, // HapticManager is !Send (contains FFI pointer)
}

impl BevyHapticManager {
    pub fn new() -> Result<Self, String> {
        let hm = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| HapticManager::new(None)))
            .map_err(|_| "Failed to initialize haptic output".to_string())?;
        Ok(Self { inner: hm })
    }

    pub fn vibrate(&mut self, millis: u32, intensity: f32) {
        self.inner.vibrate(millis, intensity);
    }
}
```

### HapticEvent Enum (Bevy Event)

Define a cross-platform event enum that carries no FFI types. Derive `Event` for Bevy's observer system, plus `Debug`, `Clone`, `PartialEq`.

```rust
#[derive(Event, Debug, Clone, PartialEq)]
pub enum HapticEvent {
    Move,       // 30ms, 0.5 intensity (grid snap feedback)
    Checkpoint, // 50ms, 0.3 intensity (save reached)
    Trap,       // 200ms, 1.0 intensity (death/eliminaton)
    Button,     // 20ms, 0.5 intensity (UI confirmation)
}
```

### Observer for Event Playback

Use `NonSendMut<BevyHapticManager>` in the observer to get exclusive main-thread access. Match on the event and delegate to the wrapped manager. **Important:** In Bevy 0.18, there is no `EventReader` — observers use `On<EventType>` as the first parameter and `commands.trigger(Event::...)` to emit.

```rust
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
```

### Plugin Build with Graceful Degradation

In `Plugin::build`, attempt initialization. On failure, log a warning and skip observer registration — haptics are non-critical. On success, insert the resource and register the observer.

```rust
impl Plugin for HapticPlugin {
    fn build(&self, app: &mut App) {
        match BevyHapticManager::new() {
            Ok(haptic_manager) => {
                app.insert_non_send_resource(haptic_manager);
                app.add_observer(haptic_playback_observer);
            }
            Err(e) => {
                log::warn!(
                    "[Haptics] Failed to initialize (non-critical): {}. Continuing without haptics.",
                    e
                );
            }
        }
    }
}
```

### iOS FFI (objc2_core_haptics)

The platform haptic implementation uses `CHHapticEngine` from `objc2_core_haptics`. The engine is wrapped in `Option` so Android can be a clean no-op.

```rust
#[cfg(target_os = "ios")]
mod platform_haptic {
    use objc2_core_haptics::{CHHapticEngine, CHHapticEvent, CHHapticEventParameters};

    pub struct HapticManager {
        engine: Option<CHHapticEngine>,
    }

    impl HapticManager {
        pub fn new(_haptic: Option<sdl2::HapticSubsystem>) -> Result<Self, String> {
            // Ignore SDL haptic — use native Core Haptics instead.
            // SDL_haptic is a no-op on iOS per SDL2 docs.
            match CHHapticEngine::new() {
                Ok(engine) => Ok(Self { engine: Some(engine) }),
                Err(e) => Err(format!("Core Haptics unavailable: {:?}", e)),
            }
        }

        pub fn vibrate(&mut self, millis: u32, intensity: f32) {
            if let Some(ref mut eng) = self.engine {
                let params = CHHapticEventParameters::new(intensity, 0.5);
                let evt = CHHapticEvent::new(
                    objc2_core_haptics::CHHapticEvent::FIELD_RUMBLE,
                    &params,
                    0.0,
                    millis as f32 / 1000.0,
                );
                let _ = eng.start();
                let _ = eng.send_events(&[evt]);
            }
        }
    }
}
```

### Triggering Haptics from Game Systems

Game systems emit `HapticEvent` via `commands.trigger(...)`. They have no dependency on the haptic plugin or platform FFI — they simply describe what happened in game terms.

```rust
// In player_movement_system — after successful move:
commands.trigger(HapticEvent::Move);

// After trap triggered:
commands.trigger(HapticEvent::Trap);

// After checkpoint reached:
commands.trigger(HapticEvent::Checkpoint);
```

## Why This Matters

- **Thread safety via non-send**: `objc2_core_haptics::CHHapticEngine` is an Objective-C object not `Send`-safe. By wrapping it in `BevyHapticManager` (which is also `!Send`) and using `insert_non_send_resource`, Bevy's type system enforces main-thread-only access. `NonSendMut` in the observer provides compile-time guarantee that the observer runs on the main thread.

- **Graceful degradation**: Haptics are purely additive (vibration feedback). If initialization fails (simulator, unsupported device, panic in FFI), the game continues without haptics. The warning log is visible in developer tools but does not crash or assert.

- **Observer decoupling**: Game systems emit semantic `HapticEvent` values (`Move`, `Trap`, `Checkpoint`). The haptic plugin observes these and translates them to platform-specific vibration calls. The two halves are decoupled — adding haptics to a new game event requires only emitting a new variant or using an existing one, not touching the FFI layer.

- **`catch_unwind` around FFI init**: Objective-C initializers can panic (e.g., if Core Haptics is unavailable on a simulator). Rust's panic cannot cross the FFI boundary safely, so catching unwinds at the Rust entry point prevents aborting the process.

## When to Apply

- **Mobile FFI + Bevy**: Any time you are wrapping a platform API (Core Haptics, GameKit, Android NDK) that is not `Send`-safe, use this non-send resource pattern.

- **`!Send` resources**: When the underlying resource is tied to the main thread or a thread-local state (FFI handles, GUI toolkit contexts, audio backends like rodio's `OutputStream`).

- **Observer for event-driven plugins**: When a Bevy plugin needs to react to game events without being in the same system or having direct query access to the triggering entities.

- **Non-critical optional features**: When a feature (haptics, certain audio, optional sensors) should not block the game if unavailable, initialize in `Plugin::build` with a fallback warning.

- **Cross-platform abstraction with platform-specific implementations**: Use `#[cfg(target_os = "...")]` modules for FFI-backed implementations and no-op stubs for unsupported platforms, with a shared public API.

## Examples

### Triggering Haptics on Player Movement (player_movement_system)

```rust
// src/bevy_plugins/player.rs — add to existing event trigger spots
use crate::bevy_plugins::haptic::HapticEvent;

// After AudioEvent::Step in player_movement_system:
commands.trigger(HapticEvent::Move);
// After AudioEvent::Trap:
commands.trigger(HapticEvent::Trap);
// After AudioEvent::Checkpoint:
commands.trigger(HapticEvent::Checkpoint);
```

### Plugin Registration (bevy_plugins/mod.rs)

```rust
pub struct BevySpritePlugin;

impl Plugin for BevySpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SpritePlugin::new(),
            IsoCameraPlugin::new(1280.0, 720.0),
            SpriteSpawnPlugin::new(),
        ));
        app.add_plugins((
            WorldPlugin::new(),
            PlayerPlugin::new(),
            NpcPlugin::new(),
            ParticlePlugin::new(),
        ));
        // Phase 4: Audio
        app.add_plugins(AudioPlugin::new());
        // Phase 5: Mobile Haptics
        app.add_plugins(HapticPlugin::new());
    }
}
```

## Related

- `src/bevy_plugins/audio.rs` — Phase 4 AudioPlugin; the primary pattern that Phase 5 followed exactly
- `src/platform/haptic.rs` — platform implementations (iOS FFI via objc2_core_haptics, Android no-op stub)
- `src/bevy_plugins/player.rs` — `player_movement_system` triggering `HapticEvent`
- `docs/solutions/integration-issues/bevy-phase3-plugin-wiring-2026-03-30.md` — Phase 3 plugin wiring integration gaps (same problem class: Bevy plugin composition)
