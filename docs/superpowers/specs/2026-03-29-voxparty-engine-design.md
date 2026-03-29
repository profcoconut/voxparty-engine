# VoxParty Engine — Minimal Isometric Pixel Game Engine

**Date:** 2026-03-29
**Status:** Draft
**Type:** Technical Design

---

## 1. Overview

A lightweight, custom Rust game engine for VoxParty — a mobile-first isometric pixel party game platform. The engine renders 2D sprites in isometric projection, supports 2-player local multiplayer on a single device, and targets iOS and Android with a shared Rust codebase.

**Guiding constraint:** Keep it minimal, like a 1980s console game. No ECS, no physics engine, no asset pipeline bloat. Just the primitives needed for a chaotic, fun isometric platformer.

---

## 2. Platform Targets

| Target | Toolchain | Notes |
|--------|-----------|-------|
| macOS | `cargo build --target x86_64-apple-darwin` | Dev/testing on desktop |
| iOS | `cargo build --target aarch64-apple-ios` | Native Rust, no Swift interop needed for v1 |
| Android | `cargo build --target aarch64-linux-android` | Native Rust via JNI or raw native activity |

**Runtime dependencies per platform:**
- **macOS:** SDL2 (via Homebrew), Cocoa framework
- **iOS:** SDL2 (compiled into app bundle), UIKit
- **Android:** SDL2 (compiled into APK), Android NDK

---

## 3. Architecture

```
app/
├── src/
│   ├── main.rs              # macOS entry point
│   ├── lib.rs               # iOS/Android entry point (C FFI)
│   ├── platform/
│   │   ├── mod.rs
│   │   ├── sdl2.rs         # SDL2 wrapper (window, events, timing)
│   │   ├── audio.rs        # Audio playback via SDL2_mixer or rodio
│   │   ├── touch.rs        # Multi-touch input mapping
│   │   └── mobile.rs       # Platform-specific: iOS/Android lifecycle
│   ├── core/
│   │   ├── mod.rs
│   │   ├── isom.rs         # Isometric coordinate math
│   │   ├── camera.rs       # Fixed camera, smooth follow player
│   │   ├── sprites.rs       # Sprite sheet loading, frame animation
│   │   ├── collision.rs     # AABB collision against tile grid
│   │   ├── scene.rs         # Game state machine
│   │   └── input.rs         # Game input abstraction
│   ├── game/
│   │   ├── mod.rs
│   │   ├── player.rs        # Player entity, state, animation
│   │   ├── world.rs         # Tile map, trap map, checkpoint map
│   │   ├── npc.rs           # NPC entity, dialogue state
│   │   ├── episode.rs       # Load episode JSON → world
│   │   └── audio.rs         # Sound effect playback
│   └── assets/
│       ├── mod.rs
│       └── loader.rs        # Load PNG + JSON sprite descriptors
├── assets/
│   ├── sprites/             # PNG sprite sheets
│   ├── sounds/              # WAV/MP3 files
│   └── episodes/            # JSON episode files
└── build.rs                 # Build script for assets embedding
```

---

## 4. Isometric Rendering

### Coordinate System

Classic 2:1 isometric projection (64x32 tile ratio):

```rust
// Grid (x, y) → screen (px, py)
fn grid_to_screen(gx: f32, gy: f32, tile_w: f32, tile_h: f32, cam_x: f32, cam_y: f32) -> (f32, f32) {
    let px = (gx - gy) * (tile_w / 2.0) - cam_x;
    let py = (gx + gy) * (tile_h / 2.0) - cam_y;
    (px, py)
}

// Screen (px, py) → grid (x, y) — inverse
fn screen_to_grid(px: f32, py: f32, tile_w: f32, tile_h: f32, cam_x: f32, cam_y: f32) -> (f32, f32) {
    let gx = (px + cam_x) / (tile_w / 2.0) + (py + cam_y) / (tile_h / 2.0);
    let gy = (px + cam_x) / (tile_w / 2.0) - (py + cam_y) / (tile_h / 2.0);
    (gx / 2.0, gy / 2.0)
}
```

### Depth Sorting

Draw all renderable objects sorted by ascending `grid_x + grid_y`. Tiles and sprites on the same depth line are drawn in order: tiles first, then sprites at that depth.

### Camera

- Fixed at game start, smooth-follows player 1's grid position
- Camera has deadzone: if player stays within center 40% of screen, camera doesn't move
- Camera bounded to world edges (no black borders at map edges)
- No zoom, no rotation

---

## 5. Sprites

### Sprite Sheet Format

PNG sprite sheets with a JSON descriptor:

```json
{
  "meta": {
    "image": "player.png",
    "tile_w": 64,
    "tile_h": 64
  },
  "frames": {
    "player_idle_0": { "x": 0, "y": 0, "w": 64, "h": 64 },
    "player_idle_1": { "x": 64, "y": 0, "w": 64, "h": 64 },
    "player_run_0":  { "x": 0, "y": 64, "w": 64, "h": 64 }
  },
  "animations": {
    "idle": { "frames": ["player_idle_0", "player_idle_1"], "fps": 2 },
    "run":  { "frames": ["player_run_0", "player_run_1"], "fps": 8 }
  }
}
```

### Animation

- Each entity holds current animation name + frame timer
- Frame rate is per-animation, not global
- Animations can be changed at any time (e.g., idle → run on movement)

---

## 6. Collision

### Tile Collision

- Each tile has a collision flag: passable or solid
- AABB check: player bounding box vs tile bounding boxes in the 3x3 grid around the player
- Grid-snapped movement: player occupies exactly one tile at a time
- On attempting to enter a solid tile, movement is blocked

### Trap Collision

- Trap tiles trigger elimination when a player enters them
- Traps have a cooldown period (they reset after N seconds)

### Checkpoint Collision

- Entering a checkpoint tile marks it as "reached" for the player
- On death, player respawns at last reached checkpoint

---

## 7. Input

### Virtual Gamepad (touch)

Two virtual gamepads displayed on screen:

```
┌─────────────────────────┬─────────────────────────┐
│   PLAYER 1              │   PLAYER 2               │
│   Left side of screen   │   Right side of screen  │
│                          │                          │
│   ┌─────┐                │                ┌─────┐  │
│   │  ○  │  joystick      │     joystick  │  ○  │  │
│   └─────┘  (deadzone 0.3)│                └─────┘  │
│                          │                          │
│   [A] Jump               │              [A] Jump   │
│   [B] Interact            │              [B] Interact│
└─────────────────────────┴─────────────────────────┘
```

- **Joystick:** Touch inside left/right half, drag to set direction. Direction quantized to 8-way (N, NE, E, SE, S, SW, W, NW).
- **Button A:** Jump — applies upward velocity impulse to player
- **Button B:** Interact — triggers checkpoint/NPC interaction if in range

### Input Mapping

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameInput {
    MoveLeft,
    MoveRight,
    MoveUp,    // north on isometric grid (screen up-left)
    MoveDown,  // south on isometric grid (screen down-right)
    Jump,
    Interact,
}
```

---

## 8. Episode Format

Episodes are JSON files loaded at runtime:

```json
{
  "id": "ep_abc123",
  "title": "Lava Panic",
  "mode": "last_standing",
  "theme": "volcanic",
  "duration_target_seconds": 540,
  "tile_width": 64,
  "tile_height": 32,
  "grid_width": 32,
  "grid_height": 32,
  "tiles": [
    { "x": 0, "y": 0, "type": "grass_solid" },
    { "x": 1, "y": 0, "type": "grass_passable" },
    { "x": 2, "y": 0, "type": "lava_trap" }
  ],
  "npcs": [
    { "x": 5, "y": 5, "name": "Coco", "dialogue": ["Hey! Watch out!", "You got this!"] }
  ],
  "checkpoints": [
    { "x": 10, "y": 0 },
    { "x": 20, "y": 0 }
  ],
  "spawn_points": [
    { "x": 1, "y": 1, "player": 1 },
    { "x": 1, "y": 3, "player": 2 }
  ],
  "win_condition": { "type": "last_player_standing" },
  "fail_condition": { "type": "fall_off_map" }
}
```

### Tile Types

| Type prefix | Behavior |
|-------------|----------|
| `*_solid` | Player cannot enter, blocks movement |
| `*_passable` | Player can walk on it |
| `*_trap` | Eliminates player on entry, then cooldown |
| `*_checkpoint` | Marks spawn point on entry |
| `*_goal` | Triggers win on entry |

---

## 9. Audio

Minimal audio playback using `rodio` (Rust audio library):

- Background music: one looping track per episode (MP3/OGG)
- Sound effects: WAV files, played on demand
  - `jump.wav` — on jump
  - `eliminate.wav` — on trap trigger
  - `checkpoint.wav` — on checkpoint reached
  - `win.wav` / `fail.wav` — on game end

---

## 10. Game State Machine

```
┌─────────────┐
│   MENU      │  Episode select, start game
└──────┬──────┘
       │ start
┌──────▼──────┐
│  TITLE CARD │  Episode splash art, 3 seconds
└──────┬──────┘
       │ (fade out)
┌──────▼──────┐
│   PLAYING   │  Main game loop
│             │  - update players, NPCs
│             │  - check win/fail conditions
│             │  - render world + entities
└──────┬──────┘
       │ win/fail triggered
┌──────▼──────┐
│  GAME OVER  │  Win/fail screen, play again or menu
└─────────────┘
```

---

## 11. Build & Asset Pipeline

### Building

```bash
# macOS
cargo build --target x86_64-apple-darwin

# iOS (requires iOS SDK + rust target)
cargo build --target aarch64-apple-ios

# Android (requires Android NDK + rust target)
cargo build --target aarch64-linux-android
```

### Asset Embedding

For simplicity, assets are embedded at compile time via `build.rs` and `include_bytes!`. Episodes and sprites are read from `assets/` at build time.

### Rust Targets Installation

```bash
rustup target add aarch64-apple-ios
rustup target add aarch64-linux-android
rustup target add x86_64-apple-darwin
```

---

## 12. Dependencies (Rust Crates)

| Crate | Version | Purpose |
|-------|---------|---------|
| `sdl2` | 0.37+ | Window, events, rendering (OpenGL ES on mobile) |
| `serde` | 1.0 | JSON episode parsing |
| `serde_json` | 1.0 | JSON parsing |
| `serde_derive` | 1.0 | Derive macros for serde |
| `image` | 0.25 | PNG loading for sprites |
| `rodio` | 0.19 | Audio playback |
| `log` + `env_logger` | any | Debug logging |
| `cfg-if` | 1.0 | Platform-specific conditional compilation |

---

## 13. Open Questions (Deferred)

1. **Multiplayer:** v1 is 2-player local. Multiplayer via Colyseus is deferred to v2.
2. **AI systems:** NPC writer and balance checker are deferred to v2.
3. **Creator editor:** Web-based editor is deferred to v2.
4. **Persistent storage:** Player progress, episode storage on device — deferred.
5. **Network sync:** Session server (Colyseus) — deferred.

---

## 14. Success Criteria for v1 Engine

- [ ] Isometric tile map renders at 60fps on iOS and Android
- [ ] Two players can move independently using touch controls
- [ ] Grid-snapped movement with 8-direction facing
- [ ] Collision with solid tiles works correctly
- [ ] Traps eliminate players, respawn at checkpoint
- [ ] NPCs display dialogue on interact
- [ ] Win condition (reach goal tile) and fail condition (fall off / lava) work
- [ ] Title card and game over screens display
- [ ] Audio plays (jump sounds, background music)
- [ ] Episode JSON loads and renders a map
