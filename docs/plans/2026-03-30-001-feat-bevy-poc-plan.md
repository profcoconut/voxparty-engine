---
title: Minimal Bevy Isometric Pixel Game POC
type: feat
status: active
date: 2026-03-30
---

# Minimal Bevy Isometric Pixel Game POC

## Overview

Build a minimal, playable isometric pixel game using Bevy 0.18. The goal is a working end-to-end demo: an isometric grid, a movable player sprite, and Bevy's built-in visual debug tools. No audio, no haptics, no episodes, no NPCs — just the core loop running and debuggable.

## Problem Frame

The previous VoxParty engine became too complex to verify as playable. The new approach: start from a provably working minimal POC, then layer complexity. If you can't see it run and debug it visually, you haven't shipped.

## Requirements Trace

- R1. Game window opens and renders an isometric tile grid
- R2. Player entity moves on a grid via WASD/arrow keys
- R3. Camera follows the player smoothly
- R4. FPS counter and debug text are visible in-game
- R5. Build passes with `cargo build`; tests pass with `cargo test`

## Scope Boundaries

**In scope:** Minimal Bevy app, isometric grid rendering, player movement, camera follow, debug overlays.
**Out of scope:** Audio, haptics, episodes, NPCs, mobile, saves, input recording. These are Phase 2+.

## Key Technical Decisions

- **Bevy 0.18** — latest stable, using `bevy::prelude::*`
- **Pixel-perfect rendering** — `TextureAtlas` with `nearest` filter, `UiScale` set to pixel ratio, no anti-aliasing
- **Isometric projection** — classic 2:1 (tile width = 2x height), `grid_to_screen(x, y) -> (sx, sy)` math using integer arithmetic
- **Grid-snapped movement** — player moves one tile per key press, with a cooldown between moves
- **Debug via Bevy inspector** — `bevy_inspector` for entity/component inspection; custom debug text for FPS and game state
- **No asset files** — all sprites are procedural (colored quads generated in code) so the POC has zero external dependencies beyond Bevy

## Implementation Units

- [ ] **Unit 1: Bevy App Scaffold + Pixel-Perfect Window**

**Goal:** A running Bevy app that opens a window, renders at pixel-perfect resolution, and logs startup.

**Requirements:** R5

**Dependencies:** None

**Files:**
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Modify: `Cargo.toml`

**Approach:**
- `bevy = "0.18"` with default features
- WindowDescriptor: 640x360 logical resolution, pixel ratio 1.0, no vsync for max debug visibility
- `App::new().run()` skeleton that logs "POC running" to stdout
- Custom `CLEAR_COLOR` to dark gray so tile grid pops

**Test scenarios:**
- Happy path: `cargo run --bin bevy_poc` opens a visible window for 2 seconds then exits cleanly
- Error path: Missing display (headless) — app logs error and exits with Ok, not panic

**Verification:**
- Window opens and displays dark gray background
- Process exits cleanly with code 0

---

- [ ] **Unit 2: Isometric Grid Rendering**

**Goal:** A 10x10 grid of isometric tiles renders on screen using colored quads.

**Requirements:** R1

**Dependencies:** Unit 1

**Files:**
- Create: `src/iso.rs` — `grid_to_screen(x, y) -> (f32, f32)`, `screen_to_grid(sx, sy, cam) -> (i32, i32)`
- Create: `src/tile.rs` — `TileType` enum (Grass, Wall), `TileBundle`, `tile_marker` component
- Create: `src/systems/render_grid.rs` — `spawn_grid_system` that creates tile entities at start
- Modify: `src/main.rs` — add TilePlugin, add grid spawn to startup

**Approach:**
- Tiles are `Sprite` bundles with `Transform` set via `grid_to_screen`
- Depth sorting: `transform.translation.z = (gx + gy) as f32` — tiles further back drawn first
- Each tile is a 32x16 pixel quad (classic 2:1 isometric ratio)
- Grass tiles: green (#3a8c3a). Wall tiles: gray (#7a7a7a).
- Grid is hardcoded 10x10 of grass, with border walls

**Test scenarios:**
- Happy path: All 100 tiles visible in isometric diamond layout
- Visual check: z-ordering correct (tile at 0,0 drawn first, tile at 9,9 drawn last)

**Verification:**
- 100 tile entities exist in the ECS world after startup
- Tiles form a recognizable isometric diamond grid

---

- [ ] **Unit 3: Player Entity + Grid Movement**

**Goal:** A player-colored square moves on the grid in response to WASD/arrow keys.

**Requirements:** R2

**Dependencies:** Unit 2

**Files:**
- Create: `src/player.rs` — `Player` component (grid_x, grid_y, cooldown), `PlayerBundle`
- Create: `src/systems/input.rs` — `player_input_system` reads keyboard, triggers grid move
- Create: `src/systems/movement.rs` — `player_move_system` applies grid movement if cooldown is 0
- Modify: `src/main.rs` — add PlayerPlugin, spawn player entity at grid (5, 5)

**Approach:**
- `Player` component stores grid position as `i32, i32`
- `player_input_system` translates `Input<KeyCode>` into intended direction (N/S/E/W)
- `player_move_system` checks cooldown, validates move against tile type, applies move, resets cooldown to 0.2s
- On successful move: update `Transform` via `grid_to_screen`
- Only moves one tile per key press (no held-key repeat)

**Test scenarios:**
- Happy path: WASD moves player one tile in correct direction
- Edge case: Walk into wall tile — player does not move, cooldown still resets
- Edge case: Rapid key presses — only one move per 0.2s cooldown window

**Verification:**
- Player sprite visually moves on grid in response to WASD
- Player cannot walk through walls

---

- [ ] **Unit 4: Camera Follow + Visual Debug Overlays**

**Goal:** Camera smoothly follows the player. Debug text shows FPS and player grid position.

**Requirements:** R3, R4

**Dependencies:** Unit 3

**Files:**
- Create: `src/systems/camera.rs` — `camera_follow_system` lerps camera toward player
- Modify: `src/player.rs` — add `IsPlayer` marker component
- Modify: `src/main.rs` — spawn camera, add debug text

**Approach:**
- Camera entity with `OrthographicProjection` + `Transform`
- `camera_follow_system`: `transform.translation.x += (target.x - transform.translation.x) * 0.1`
- Debug text using `TextBundle` with `Style` anchored to top-left
- Shows: "FPS: {fps} | Player: ({gx}, {gy})"
- Update text in `player_move_system` or a separate `debug_text_system`

**Test scenarios:**
- Happy path: Camera follows player smoothly without jarring
- Debug text updates every frame showing correct FPS and grid position

**Verification:**
- Debug text visible in top-left corner
- Camera does not snap (lerp visible as smooth drift)

---

- [ ] **Unit 5: Smoke Test + cargo build**

**Goal:** Verify the full POC builds and runs without panic.

**Requirements:** R5

**Dependencies:** Unit 4

**Files:**
- Modify: `Cargo.toml` — add `[[bin]]` for `bevy_poc`
- Create: `src/bin/poc_smoke_test.rs` — headless test that starts app, waits 1s, checks no panic

**Approach:**
- `cargo build` must produce a working binary with zero warnings (or only known-safe warnings)
- Smoke test binary: starts app in headless mode, verifies no panic after 60 frames worth of time, exits cleanly
- Add `cargo test` integration: runs the smoke test as a unit test

**Test scenarios:**
- `cargo build` — zero errors
- `cargo test` — smoke test passes (no panic, clean exit)
- Headless: `cargo run --bin poc_smoke_test` — exits 0 within 5 seconds

**Verification:**
- `cargo build && cargo test` exits 0
- Binary runs headless without display and exits cleanly

## System-Wide Impact

This is a greenfield project. No existing system to impact.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Bevy 0.18 API for spawn/movement differs from expectations | Use `bevy::prelude::*` and follow Bevy examples; check bevy.io for minimal spawn patterns |
| Pixel-perfect rendering needs specific settings | Set `TextureAtlas` filter to `nearest`, disable anti-aliasing in `RenderGraph` |
| Isometric depth sorting wrong | Use `gx + gy` as z-translation; verify visually (tile at 0,0 behind tile at 9,9) |

## Documentation / Operational Notes

- `README.md` in worktree root: how to build and run
- Debug keys: no additional debug keys in Sprint 1 — debug info always visible via overlay
- Future: WASD movement debug, collision step-through
