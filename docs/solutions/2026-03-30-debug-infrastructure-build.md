---
name: Debug Infrastructure Build (Sprints 15-22)
description: How we built a complete debug infrastructure layer for VoxParty — overlay, console, assertions, screenshots, telemetry, QA, and profiling
type: sprint-compound
sprints: [15, 16, 17, 18, 19, 20, 21, 22]
test_count: 160
completed_at: 2026-03-30T10:55:00Z
---

# Debug Infrastructure Build (Sprints 15-22)

## What We Built

A complete debug infrastructure layer for VoxParty, enabling runtime state inspection, automated QA, and mobile profiling. Five interconnected sprints that grew from a single FPS counter into a full telemetry + QA + profiling system.

## The Problem

The VoxParty engine had no runtime introspection. Debugging meant guesswork — is the camera following? Are players on-screen? Why did the game lag? No logs, no assertions, no replay.

## Approach

Built layer by layer, each sprint depending on the previous:

1. **Overlay + Assertions (Sprint 15)**: Human-readable debug overlay with 6 sections, 8x12 bitmap font, HealthMonitor with camera/FPS/player-onscreen checks
2. **In-Game Console (Sprint 16)**: Quake-style tilde console with 9 commands (cam, p1, p2, scene, god, state, fps, screenshot, exit)
3. **Screenshot-on-Bug (Sprint 17)**: HealthMonitor FAIL → automatic PNG saved to `screenshots/bug_<ts>.png`
4. **Structured Telemetry (Sprint 18)**: `TelemetryLogger` → `telemetry/session_<ts>.jsonl`, events: scene_change, player_move, player_die, checkpoint, trap, fps_sample, health_fail
5. **Input Replay + QA (Sprint 19)**: Headless replay with invariant assertions (camera<200px, fps>=10, no_player_deadlock), `--repro` bug reproduction, QA report JSON
6. **Visual Polish (Sprint 20)**: Isometric minimap (F4 cycle), death recap flash, particle count tuning
7. **Mobile CI (Sprint 21)**: GitHub Actions iOS/Android builds, `--headless` flag, smoke_test binary
8. **Mobile Profiling (Sprint 22)**: Frame time RingBuffer (F5), particle cap 200, texture memory audit, draw call counting

## Key Decisions

### VecDeque over Vec for O(1) history trimming
Console history and output used `Vec::remove(0)` which is O(n). Changed to `VecDeque::pop_front()` — O(1). Same for telemetry event buffering (64-line flush threshold).

### Camera staleness via cumulative player distance
Original camera staleness check compared player position between frames — but when camera wasn't moving, player movement was invisible. Fixed by accumulating `cumulative_player_dist` while `!cam_following`, triggering WARN at 30 frames and FAIL at 60.

### HealthMonitor as single source of truth for assertions
All runtime assertions (camera, FPS, player position) live in `HealthMonitor::check()`. Screenshot-on-bug and telemetry both read from this single source rather than duplicating checks.

### JSONL over JSON for telemetry
Streaming format avoids large in-memory serialization. Write errors are non-fatal — logged and continued. Session file is append-only.

### RingBuffer for frame times
Fixed-size `RingBuffer<f32, 120>` (2 seconds at 60fps) with O(1) insert. P99 computed from sorted copy on read. No allocations after initialization.

### AtomicUsize for draw call counting
Global `AtomicUsize` incremented in `blit_sprite()` via `fetch_add(SeqCst)`. Reset at frame start, read before present. Batched approach avoids per-call synchronization overhead.

## What We'd Do Differently

1. **Plan telemetry before building it**: Sprint 18's telemetry format didn't anticipate the QA replay needs from Sprint 19. Should have designed the event schema together.

2. **Particle budget was a mid-sprint change**: Sprint 20's particle count increase (8→20 for traps) broke existing tests. Should have locked particle constants before implementing.

3. **F4/F5 state machine should be an enum**: Using bools for F4 cycle (FPS/minimap/both/off) works but an enum would be cleaner and extensible.

4. **Headless mode needs more investment**: The `--headless` flag works but doesn't yet run in CI. Should add a proper headless test runner that exercises all 3 episodes.

## Patterns Established

- **HealthMonitor pattern**: `check()` method that populates `results` struct, with optional side effects (screenshot reason). Separates detection from response.
- **Overlay toggle cycling**: F3/F4/F5 each own a bit of state, cycling through 4 modes independently.
- **Telemetry event emission**: `log_event(event, payload)` with JSON payload, buffered 64 lines, flush on drop.
- **Dedicated simplify pass**: Every sprint has a parallel simplify review that catches dead code, unused imports, and performance issues.

## Files Changed

- `src/core/debug.rs`: 8x12 font, 6-section overlay, RingBuffer, frame graph, minimap
- `src/core/health.rs`: HealthMonitor, CheckResult, screenshot reason
- `src/core/console.rs`: Console with VecDeque history/output
- `src/core/telemetry.rs`: TelemetryLogger, JSONL session files
- `src/game/input.rs`: InputRecorder, InputReplayer, headless_replay, assertions
- `src/lib.rs`: Console wiring, telemetry emission, F3/F4/F5 cycles, headless mode
- `src/core/scene.rs`: Death recap
- `src/core/particles.rs`: Particle budget cap 200
- `src/core/sprites.rs`: Texture memory audit, draw call counting
- `.github/workflows/mobile-builds.yml`: iOS + Android CI
- `src/bin/smoke_test.rs`: Headless smoke test binary
