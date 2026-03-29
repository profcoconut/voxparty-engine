# Sprint 2 Solutions: demo-polish

**Date:** 2026-03-29
**Sprint:** demo-polish
**Status:** All 8 items implemented

---

## Overview

Sprint 2 focused on making the VoxParty demo genuinely impressive through synthesized audio, visual polish (particles, animations), more content (episode 2), and debug tooling. All work is self-contained in the Rust engine with no external asset dependencies.

---

## Solutions

### sfx-1: Synthesized Sound Effects

**Problem:** Need impactful sound effects without audio file dependencies.

**Solution:** All SFX synthesized via `rodio::Source::SineWave` with envelope shaping:

| Event    | Frequency(s)         | Duration | Character           |
|----------|----------------------|----------|---------------------|
| Move     | 880 Hz               | 50ms     | Quick blip          |
| Trap     | 400 + 420 Hz (beat)  | 300ms    | Detuned clash + thud |
| Checkpoint | C5→E5→G5 (arpeggio) | 400ms    | Ascending chime     |
| Victory  | C→E→G→C (octave run) | 800ms    | Triumphant fanfare  |

Envelope: 10ms attack, sustain, 50ms release. Fallback to no-op when audio subsystem unavailable.

---

### episode-2: Volcanic Underground Episode

**Problem:** Demo needed a second distinct environment beyond the starter map.

**Solution:** 20x20 volcanic map with:

- `stone_solid` — impassable walls forming cavern passages
- `lava_trap` — 2-tile-wide river of instant-elimination tiles
- `bridge_passable` — walkable stone bridges over lava
- 3 checkpoints with NPC dialogue at each
- Goal tile at far end

No new tile types added to the engine — all reuse existing tile traits.

---

### particle-1: Particle System

**Problem:** Visual juice needed beyond static sprites.

**Solution:** `ParticleSystem` with 3 types, tick/draw each frame:

| Type          | Color    | Behavior                        | Lifetime |
|---------------|----------|----------------------------------|----------|
| Sparkle       | Gold (#FFD700) | Burst upward, gravity fall   | 0.8s     |
| TrapFlash     | Red (#FF0000) | Ring expand outward           | 0.3s     |
| MovementDust  | Grey (#888888) | Puff at player feet, fade   | 0.4s     |

Each particle has: position, velocity, acceleration, lifetime, alpha fade. Spawned from game events (move, trap trigger, checkpoint).

---

### pause-1: Pause State

**Problem:** No way to pause mid-game; ESC/Q had no effect.

**Solution:** `SceneState::Paused` variant added to scene state machine:

```
SceneState = Menu | TitleCard | Playing | Paused | GameOver
```

- ESC toggles `scene.toggle_pause()`
- Q returns to menu from pause
- Paused state: dark overlay drawn, all input intercepted
- Toggle preserves game state (no reset)

---

### haptics-1: Mobile Haptic Feedback

**Problem:** Mobile lacked tactile feedback on events.

**Solution:** `HapticManager` wrapping SDL_haptic:

- Checkpoint: 50ms, intensity 0.3
- Trap: 200ms, intensity 1.0
- Desktop: no-op (compile-time conditional)

```rust
impl HapticManager {
    fn checkpoint(&self) { /* SDL_haptic rumble 50ms/0.3 */ }
    fn trap(&self)       { /* SDL_haptic rumble 200ms/1.0 */ }
}
```

---

### titlecard-anim-1: Title Card Animation

**Problem:** Title cards were static; no sense of anticipation.

**Solution:** Two timers on title card:

- `fade_timer` (1.0s): episode name fades from alpha=0 to alpha=1
- `pulse_timer` (0.8s cycle): "GET READY" pulses opacity 0.4→1.0→0.4

Both timers reset on episode start. Rendering draws overlay then text with current alpha.

---

### gamepad-full-1: Full Gamepad Support

**Problem:** Only menu navigation; Start/B buttons not wired.

**Solution:** Static bool tracking in input handler:

```rust
// Static press detection (not edge-triggered)
if gamepad.start_button_pressed() {
    scene.toggle_pause();
}
if gamepad.b_button_pressed() {
    scene.return_to_menu();
}
```

Start = pause toggle. B = return to menu. Static tracking prevents double-fire from held buttons.

---

### debug-screenshot-1: Screenshot Tool

**Problem:** No way to capture game state for debugging.

**Solution:** F12 key -> `plat.screenshot()` -> `image::codecs::png::PngEncoder` -> PNG file:

```
screenshot_{timestamp}.png
```

Flash timer shows "SAVED" overlay for 1 second after capture. Uses `image` crate for encoding, `std::time::UNIX_EPOCH` for filename.

---

## Key Patterns

1. **Synthesized audio:** All SFX via `SineWave` + envelope. No audio file loading, no asset pipeline.
2. **Particles tick/draw:** Each frame calls `particles.tick(dt)` then `particles.draw()`. Lifetime-based alpha fade.
3. **Pause intercepts input:** `SceneState::Paused` checked at top of input handler, short-circuits all other input.
4. **Haptics conditional:** `#[cfg(target_os = "ios" | target_os = "android")]` compiles out on desktop.
5. **Static gamepad tracking:** Bool fields updated each frame; press = currently held (not edge-detected).

---

## Test Coverage

- Tests: 97 → 103 (+6)
- New tests cover: player movement, world tile access, scene pause/resume, episode deserialization
