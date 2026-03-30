# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0.0] - 2026-03-30

### Added

- **Bevy 0.18 Engine Scaffold (Phase 1):** Initial Bevy app setup with window creation,
  input handling, and SDL2 integration. `BevySpritePlugin` orchestrates all Bevy plugins.
- **Sprite Rendering (Phase 2):** `TextureAtlas` sprite sheet loading, `GridPos` component
  for isometric grid positioning, `IsoCameraBundle` with smooth-follow and screen shake,
  depth-sorted isometric rendering (back-to-front). `SpriteSpawnPlugin` handles tile and
  player entity spawning.
- **ECS Components (Phase 3):** `PlayerComponent` with grid movement, cooldown, and
  checkpoint tracking. `NpcComponent` with dialogue state machine. `WorldComponent` for
  tile map with trap/checkpoint/goal tracking. `Particle` ECS components and `ParticleSystem`
  with sparkle, flash, and dust effects.
- **Audio Plugin (Phase 4):** `AudioEvent` enum (Move, Step, Trap, Checkpoint, Victory,
  GameOver, MusicEpisode) triggered via Bevy observers. `BevyAudioManager` wraps
  `rodio`-based `AudioManager` as a `NonSend` resource. Material-aware synthesized
  footsteps and event-driven SFX.
- **Mobile Haptics (Phase 5):** `HapticEvent` enum (Move, Checkpoint, Trap, Button)
  triggered via Bevy observers. `BevyHapticManager` wraps platform haptic state as a
  `NonSend` resource. iOS: Core Haptics FFI via `objc2_core_haptics`. Android/desktop:
  no-op stub. Graceful degradation when haptics unavailable.
- **iOS Hello World Demo:** Minimal SwiftUI app (iOS 17, iPhone Simulator). XcodeGen-generated project (`project.yml`), interactive "Hello, World!" with tap counter, visual debug overlay showing live FPS (CADisplayLink), frame count, view depth, and device name via `utsname`.
  bridging SDL2 game loop and Bevy State API. `SceneData` resource with per-state timers.
  `TitleCardTimer` (3s), `GameOverTimer` (5s), `VictoryTimer` (10s) auto-transition via
  Bevy `Timer` resources. Transition functions: `go_to_episode_select`, `go_to_titlecard`,
  `go_to_menu`, `toggle_pause`, `trigger_victory`, `trigger_gameover`.

### Changed

- **Edition Rollback:** `edition = "2024"` reverted to `edition = "2021"` for Rust 1.85
  compatibility.
- **Game Loop Refactor:** `lib.rs` `inner_run` updated to use `GameStateSignal` for
  scene state transitions. Scene state no longer directly controls rendering — `GameState`
  Bevy enum drives schedule running.

### Fixed

- Clippy: unnecessary parentheses in touch overlay rendering.
- Clippy: removed unnecessary `mut` on `game_state_signal`, `input_recorder`.
- Clippy: silenced unused variable warnings for gamepad overlay placeholder values.

### Infrastructure

- Added `objc2-core-haptics = "0.3"` dependency for iOS Core Haptics FFI.
- Added `bevy = "0.18"` game engine dependency.
- GitHub Actions CI: `mobile-builds.yml` runs desktop build + tests on every PR.
