# Playable Demo Sprint (2026-03-29)

## What Was Solved

An 8-item sprint to make VoxParty genuinely playable as a mobile-first demo. All items were implemented in a single session by parallel workers.

## Key Solutions

### Touch Input (`touch-1`)
- **Problem**: SDL2 reports arbitrary `finger_id` values per touch, not stable indices
- **Solution**: `FingerTracker` uses a `HashMap<f64, Finger>` keyed by `x,y` position to track fingers across frames, solving the arbitrary finger_id problem
- **Result**: Multi-touch D-pad zones work correctly across two players

### Dialogue Bubbles (`dialogue-1`)
- **Problem**: `DebugOverlay::draw_text` was only printing to stderr, not rendering on screen
- **Solution**: `DebugOverlay::draw_text` now properly renders dialogue content in NPC speech bubbles
- **Pattern**: All text rendering now flows through `DebugOverlay::draw_text`

### Victory Screen (`win-1`)
- **Problem**: No distinct victory state from game over
- **Solution**: Added separate `SceneState::Victory` variant with timer display
- **Differentiation**: Victory shows completion time; GameOver shows countdown to menu return

### Gamefeel (`gamefeel-1`)
- **Problem**: Movement felt flat without feedback
- **Solution**: Camera shake via `shake_timer` + `shake_intensity` fields on Camera
- **Parameters**: 80ms duration, configurable intensity

### Animation (`anim-1`)
- **Problem**: Walk animation frames were not advancing
- **Solution**: `AnimPlayer::advance()` called in `Player::tick()` after `play()` to advance walk frames
- **Pattern**: Animation state machine drives sprite selection per tick

### HUD (`hud-1`)
- **Problem**: No visual feedback for checkpoint progress or lives
- **Solution**: Added `checkpoint_progress()` and `lives()` methods to World/Player, rendered via `DebugOverlay::draw_text`
- **Pattern**: All text rendering via DebugOverlay; all positioning via grid coordinates

### GameOver (`gameover-1`)
- **Problem**: GameOver state had no countdown feedback
- **Solution**: `gameover_timer` countdown displayed with "RETURNING TO MENU IN X..." text
- **UX**: Player knows when they will return to menu

### Audio (`audio-1`)
- **Problem**: `play_music_stub` was a no-op
- **Solution**: Synthesized ambient via layered sine waves (A2+E3+A3) with fade-in using rodio
- **Pattern**: All audio via `AudioManager` methods; synthesized audio replaces placeholder stubs

## Key Patterns Established

| Concern | Pattern |
|---------|---------|
| Text rendering | `DebugOverlay::draw_text` |
| Camera shake | `Camera::shake()` with `shake_timer` + `shake_intensity` |
| Audio | `AudioManager` methods with synthesized sounds |
| Animation | `AnimPlayer::advance()` called each tick |
| Touch tracking | `FingerTracker` HashMap by position |
| Scene states | `SceneState` enum with `Victory`, `GameOver`, etc. |

## Files Changed

- `src/core/camera.rs` - shake_timer, shake_intensity fields
- `src/game/player.rs` - animation advance, movement cooldown
- `src/game/npc.rs` - dialogue state
- `src/game/world.rs` - checkpoint/lives tracking
- `src/platform/touch.rs` - FingerTracker HashMap
- `src/platform/audio.rs` - Synthesized ambient via rodio
- `src/core/scene.rs` - Victory scene state
- `src/core/sprites.rs` - DebugOverlay text rendering

## Metrics

- Tests: 97 (all passing)
- Build: clean
- Sprint items: 8/8 implemented
