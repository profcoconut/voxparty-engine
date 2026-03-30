---
module: engine/bevy_plugins
date: 2026-03-30
category: docs/solutions/logic-errors/
problem_type: logic_error
component: tooling
symptoms:
  - Camera follow system silently failed at runtime — player sprites invisible to iso_camera_follow_system
  - iso_camera.rs defined GridPos(x: f32, y: f32, z: i32) while sprite.rs defined GridPos(x: i32, y: i32, z: i32)
  - Rust separate compilation hid the mismatch — each module compiled against its own GridPos definition
root_cause: missing_tooling
resolution_type: code_fix
severity: medium
tags:
  - parallel-agents
  - type-system
  - bevy-plugin
  - gridpos
  - sprint-23
---

# GridPos Type Divergence Between Parallel Agents in Sprint 23

## Problem

During Sprint 23, four parallel agents simultaneously created Bevy plugin files for the VoxParty engine migration. Two agents independently defined a `GridPos` component struct with incompatible field types — `iso_camera.rs` used `f32` for x/y while `sprite.rs` used `i32`. Since Bevy's ECS uses exact struct-type equality for component matching, the camera follow system could not read player sprite positions at runtime.

## Symptoms

- **Runtime-only failure**: The build succeeded because Rust's separate compilation meant each module compiled against its own local `GridPos` definition. No compile-time error surfaced.
- **Silent camera failure**: `iso_camera_follow_system` queried `Query<&GridPos, With<PlayerTag>>` and received zero results because the player sprites had `GridPos(i32)` (from `sprite_spawn.rs`) while the camera system expected `GridPos(f32)` (from `iso_camera.rs`'s local definition).
- **Build passed after fix**: After unifying on `GridPos(i32)` from `sprite.rs` and removing the duplicate f32 version, all systems worked correctly.

## What Didn't Work

- **Per-module type definitions**: Each agent independently defined `GridPos` locally rather than importing a shared canonical type. This is the root cause.
- **Separate compilation masking the mismatch**: Rust's separate compilation prevented cross-module type checking. A multi-file Rust project compiles each module independently, so type conflicts between modules only surface at link time or runtime.
- **Runtime query failure**: Bevy's component queries returned no matches silently — there was no panic, just an empty query result that caused the camera to not follow players.

## Solution

**Before** — `iso_camera.rs` had its own duplicate `GridPos`:

```rust
// iso_camera.rs — duplicate type (f32 vs sprite.rs i32)
#[derive(Debug, Clone, Copy, Component)]
pub struct GridPos {
    pub x: f32,  // <-- different from sprite.rs
    pub y: f32,  // <-- different from sprite.rs
    pub z: i32,
}
```

**After** — `iso_camera.rs` imports the canonical `GridPos` from `sprite.rs`:

```rust
// iso_camera.rs — imports canonical GridPos
use crate::bevy_plugins::sprite::GridPos;
```

**Canonical `GridPos` lives in `sprite.rs`** (single source of truth):

```rust
// sprite.rs — single source of truth
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
```

**Camera follow system casts i32 to f32 for `grid_to_screen`**:

```rust
// iso_camera_follow_system
let (sx, sy) = grid_to_screen(player_pos.x as f32, player_pos.y as f32, 0.0, 0.0);
```

**`mod.rs` re-exports `GridPos` as the canonical type**:

```rust
// bevy_plugins/mod.rs
pub use sprite::{GridPos, SortedSprite, SpritePlugin};
```

## Why This Works

Bevy's ECS requires exact type identity for query matching — not just structural compatibility. By consolidating `GridPos` into `sprite.rs` and re-exporting it via `mod.rs`, all systems reference the same canonical type. The `iso_camera_follow_system` correctly queries `Query<&GridPos, With<PlayerTag>>` and receives the same `GridPos(i32)` that `sprite_spawn.rs` attaches to player entities.

The `as f32` cast in the camera system is safe because grid coordinates are small integers and casting to f32 preserves full precision for isometric projection math.

## Prevention

- **Central type registry before parallel dispatch**: When spawning parallel agents, create a `src/bevy_plugins/types.rs` or use `mod.rs` re-exports as the single source of truth for all shared component/resource types *before* agents start writing code. Require all agents to import from this registry rather than define types locally.
- **Type protocol in agent briefs**: Explicitly specify which module owns each shared type in the agent prompt. For example: "GridPos is owned by sprite.rs — do not re-define it, import it."
- **Integration smoke test**: A `cargo build` + minimal Bevy app that runs one frame catches type mismatches at build time, not runtime. Add `cargo test --lib` to verify all test targets compile together.
- **Avoid duplicate systems**: The same `iso_depth_sort_system` was implemented twice (in both `sprite.rs` and `iso_camera.rs`). Use `.chain()` ordering and register each system exactly once in the combined plugin builder.

## Related Issues

- [VoxParty Bevy Migration Plan](docs/plans/2026-03-30-012-refactor-bevy-migration-plan.md) — predecessor document
- [Debug Infrastructure Build Sprint Doc](docs/solutions/2026-03-30-debug-infrastructure-build.md) — Sprint 22 sprint retrospective (multi-sprint parallel coordination pattern)
