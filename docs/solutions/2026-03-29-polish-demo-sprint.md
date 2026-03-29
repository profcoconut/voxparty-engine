# Sprint 4 Solutions: polish-demo

**Date:** 2026-03-29
**Sprint:** polish-demo
**Status:** All 6 items implemented

---

## Overview

Sprint 4 focused on making the VoxParty demo showable to a boss — zero test failures and a cohesive intentional ASCII aesthetic. All sprites are generated programmatically; no external art assets required.

---

## Solutions

### ascii-tiles-1: Programmatic Tile Sprites

**Problem:** Needed tile sprites without external PNG assets.

**Solution:** `tile_gen.rs` uses the `image` crate to generate all tile sprites in-memory:

| Tile Type       | ASCII Char | Description                     |
|-----------------|------------|----------------------------------|
| grass_passable  | `░` (U+2591) | Light fill, walkable ground     |
| grass_solid     | `▓` (U+2593) | Dense fill, blocking wall      |
| stone_solid     | `█` (U+2588) | Full block, impassable          |
| stone_passable  | `▒` (U+2592) | Medium fill, walkable floor     |
| lava_trap       | `▒` (U+2592) | Medium fill, eliminates player  |
| water_trap      | `~`        | Wavy water, eliminates          |
| checkpoint      | `◆` (U+25C6) | Saves spawn point              |
| goal            | `★` (U+2605) | Wins the level                 |
| bridge_passable | `─`        | Horizontal walkable bridge      |
| bridge_solid    | `═`        | Horizontal blocking bridge      |

Each tile renders the ASCII character centered in a 64x32 pixel tile using the `image` crate's `GenericImageView` and `ImageBuffer`.

---

### ascii-chars-1: Programmatic Character Sprites

**Problem:** Needed character sprites (player, NPC) without external assets.

**Solution:** `sprite_gen.rs` generates character sprites using the same `image` crate approach:

| Character   | Glyph | Color       | Size       |
|-------------|-------|-------------|------------|
| Player 1    | `@`   | Blue `#0000FF` | 32x32   |
| Player 2    | `@`   | Red `#FF0000` | 32x32    |
| NPC         | `?`   | Purple `#800080` | 32x32 |

Characters are rendered at 2x scale (glyph is 16x16 in a 32x32 buffer) to match the isometric tile projection.

---

### fix-tests-1: Zero Test Failures

**Problem:** Test suite had failing tests preventing clean CI.

**Solution:** Debugged and fixed root causes across multiple modules:

- `isom.rs`: Corrected `screen_to_grid` inverse formula
- `scene.rs`: Reset `winner` field in `return_to_menu()`
- `player.rs`: Fixed grid-snap movement state machine
- `world.rs`: Corrected tile access boundary checks

Result: 105 tests passing, build is clean.

---

### menu-polish-1: ASCII Menu UI

**Problem:** Menu UI was plain text; needed visual polish.

**Solution:** Box-drawing ASCII menu with:

```
┌──────────────────────────────────────┐
│           V O X P A R T Y            │
│         [ Episode Select ]           │
│                                        │
│  >> EPISODE 1 - The Beginning    ★★☆  │
│     EPISODE 2 - Volcanic         ★★★  │
│     EPISODE 3 - Crystal Caves    ★★★  │
│                                        │
│         [ Press SPACE/ENTER ]          │
└──────────────────────────────────────┘
```

- Header uses box-drawing `┌─┐│└─┘` characters
- Episode list shows theme icon + difficulty stars
- Selected item marked with `>>`
- Controls shown at bottom

---

### hud-polish-1: Bordered HUD Display

**Problem:** HUD lacked visual structure and clarity.

**Solution:** Bordered HUD with checkpoint/lives/time/best format:

```
+-- HUD ----------------------------+
| CP: 2    LIVES: 3    TIME: 42    BEST: 38 |
+-----------------------------------+
```

- Corner characters: `+--` and `--+` delimit header
- Vertical pipe `|` separates data fields
- Bottom border with `+` corners and `-` fill
- Compact single-line format fits above game area

---

### victory-ascii-1: ASCII Victory Screen

**Problem:** Victory screen needed fanfare presentation.

**Solution:** Full ASCII celebration screen:

```
+-- VICTORY --------------------------------+
|                                            |
|    ************************************    |
|    *                                      *|
|    *     CONGRATULATIONS, CHAMPION!      *|
|    *                                      *|
|    ************************************    |
|                                            |
|           EPISODE 1 COMPLETE               |
|                                            |
|           Time: 42    Best: 38             |
|                                            |
|        Returning to menu in 5...          |
|                                            |
+--------------------------------------------+
```

- Star border `*` frames the victory banner
- Winner announcement centered in ASCII box
- Episode completion + time/best stats
- Auto-countdown (5 seconds) to return to menu

---

## Key Patterns

1. **Programmatic sprite generation:** All sprites created via `image::ImageBuffer` + `image::codecs::png::PngEncoder`; no external PNG files loaded.
2. **In-memory to FFI:** `load_sprite_from_bytes()` in `sdl2.rs` takes `&[u8]` PNG data and creates `SDL_Texture` directly — no tempfile needed.
3. **ASCII box-drawing:** Consistent use of Unicode box-drawing (`┌─┐│└─┘`) and block (`░▓█▒`) characters for all UI chrome.
4. **Stateless rendering:** HUD and menu render every frame from game state; no persistent UI state.
5. **Countdown pattern:** Victory/menu transitions use `f32` timer that decrements each frame, triggers state change at 0.0.

---

## Test Coverage

- Tests: 103 → 105 (+2)
- New tests cover: isom coordinate transforms, scene state transitions

---

## Files Added/Modified

| File                    | Change        | Purpose                              |
|-------------------------|---------------|--------------------------------------|
| `src/core/tile_gen.rs`  | Added         | Programmatic tile sprite generation  |
| `src/core/sprite_gen.rs`| Added         | Programmatic character generation    |
| `src/platform/sdl2.rs`  | Modified      | Added `load_sprite_from_bytes()`     |
| `src/core/mod.rs`       | Modified      | Export new sprite gen modules        |

**Total files changed:** 10 (across all sprint items)
