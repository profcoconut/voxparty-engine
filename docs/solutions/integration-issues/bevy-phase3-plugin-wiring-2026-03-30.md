---
module: engine/bevy_plugins
date: 2026-03-30
problem_type: integration_issue
component: tooling
symptoms:
  - Phase 3 plugins not registered in BevySpritePlugin — all systems silently disconnected
  - VoxpartyWorld resource missing at runtime (only created in tests via from_episode())
  - world_tick_system and checkpoint_activation_system unregistered (no plugin owned them)
  - Camera lerp negated each frame (prev overwritten with target, not accumulated)
  - camera_shake_system and iso_camera_follow_system both modified same Transform without ordering
root_cause: missing_tooling
resolution_type: code_fix
severity: critical
tags:
  - bevy-plugin
  - ecs
  - plugin-wiring
  - parallel-agents
  - sprint-24
  - phase-3
---

# Bevy Phase 3 ECS Plugin Wiring — Integration Gaps

## Problem

Phase 3 of the Bevy migration (converting OOP tick methods to ECS systems) produced five new Bevy plugin files: `player.rs`, `npc.rs`, `world.rs`, `particle.rs`, and enhanced `iso_camera.rs`. After the parallel agents completed, all Phase 3 systems silently did not run.

## Symptoms

- **Silent failure**: Build succeeded — no compile errors. Systems simply never executed at runtime.
- **`VoxpartyWorld` resource missing**: `player_movement_system` and `tile_spawn_system` both require `Res<VoxpartyWorld>`, but the resource was only created via `VoxpartyWorld::from_episode()` in unit tests — never inserted into the Bevy app.
- **Systems unregistered**: `world_tick_system` and `checkpoint_activation_system` had no plugin calling `app.add_systems()` for them.
- **Camera snap instead of lerp**: `iso_camera_follow_system` computed a lerp toward target but then immediately overwrote `prev` with `target`, negating the smoothing.
- **Double shake**: Both `iso_camera_follow_system` and `camera_shake_system` modified the same `Camera2d` `Transform` in the same `PostUpdate` schedule without ordering constraints.

## What Didn't Work

- **Review caught it, but not at implementation time**: The review agent identified 10 critical/major issues, but the code had already been committed. Fixed post-commit.
- **Separate compilation masked the problem**: Rust's separate compilation meant `player.rs` and `npc.rs` compiled against their own modules. Cross-module integration gaps only surfaced at runtime.
- **No Bevy frame integration test**: A test that started the Bevy app and ran one frame would have caught missing resource insertions immediately.

## Solution

**Fix 1 — Wire all Phase 3 plugins into `BevySpritePlugin`** (`src/bevy_plugins/mod.rs`):

```rust
impl Plugin for BevySpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SpritePlugin::new(),
            IsoCameraPlugin::new(1280.0, 720.0),
            SpriteSpawnPlugin::new(),
            WorldPlugin::new(),      // Phase 3: VoxpartyWorld resource + world systems
            PlayerPlugin::new(),      // Phase 3: player movement
            NpcPlugin::new(),         // Phase 3: NPC dialogue
            ParticlePlugin::new(),    // Phase 3: particle systems
        ));
    }
}
```

**Fix 2 — Create `WorldPlugin` that inserts `VoxpartyWorld`** (`src/bevy_plugins/world.rs`):

```rust
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // Insert placeholder VoxpartyWorld — actual episode loading
        // replaces this via a separate system that runs on scene entry.
        let episode = Episode {
            id: "placeholder".into(),
            title: "Loading".into(),
            mode: crate::game::EpisodeMode::Solo,
            theme: crate::game::Theme::Grass,
            difficulty: crate::game::Difficulty::Easy,
            duration_target_seconds: 60,
            tile_width: 64,
            tile_height: 32,
            grid_width: 3,
            grid_height: 3,
            tiles: vec![],
            npcs: vec![],
            checkpoints: vec![],
            spawn_points: vec![],
            win_condition: crate::game::WinCondition { kind: crate::game::WinConditionKind::ReachGoal },
            fail_condition: crate::game::FailCondition { kind: crate::game::FailConditionKind::FallOffMap },
        };
        app.insert_resource(VoxpartyWorld::from_episode(episode));
        app.add_systems(PostUpdate, world_tick_system);
    }
}
```

**Fix 3 — Fix camera lerp (prev now persists between frames)** (`src/bevy_plugins/iso_camera.rs`):

```rust
// BEFORE (broken): prev overwritten each frame — camera snaps
prev.x += (target_x - prev.x) * lerp_speed;
prev.y += (target_y - prev.y) * lerp_speed;
prev.x = target_x;  // ← THIS OVERWROTE THE LERPED VALUE
prev.y = target_y;

// AFTER (fixed): lerp accumulates toward target smoothly
prev.x += (target_x - prev.x) * lerp_speed;
prev.y += (target_y - prev.y) * lerp_speed;
// prev.x/y persist and accumulate frame-to-frame
```

**Fix 4 — Add system ordering between follow and shake** (`src/bevy_plugins/iso_camera.rs`):

```rust
app.add_systems(PostUpdate, (
    iso_camera_follow_system.before(camera_shake_system),
    camera_shake_system,
));
```

**Fix 5 — Remove dead `CheckpointPos`** (`src/bevy_plugins/world.rs`): `CheckpointPos` was queried by `checkpoint_activation_system` but never inserted on any entity. `player_movement_system` already handles checkpoint logic inline. Both were removed.

## Why This Works

- **`BevySpritePlugin` is the composition root**: In Bevy, `app.add_plugins()` registers systems with the scheduler. Anything not registered via `add_plugins()` or `add_systems()` is invisible to Bevy at runtime. Adding all Phase 3 plugins to the composition root ensures their systems are scheduled.
- **`WorldPlugin` is the resource insertion point**: Every `Resource` type needs exactly one `app.insert_resource()` call in a `Plugin::build()`. Using a dedicated plugin for this makes the insertion point explicit and discoverable.
- **Lerp persistence**: The correct lerp pattern is `prev = prev + (target - prev) * speed` which converges toward target over multiple frames. Overwriting `prev = target` each frame resets the accumulation, causing snap-to-target.
- **System ordering via `.before()`**: When two systems write to the same component in the same schedule, Bevy executes them in registration order unless `.before()`/`.after()` is specified. `.before(camera_shake_system)` ensures follow runs first, then shake offsets the already-followed position.

## Prevention

1. **Bevy plugin registration checklist**: Every new plugin file that defines `impl Plugin for XPlugin` MUST be added to `BevySpritePlugin::build()` before the PR is merged. Add a comment:

   ```rust
   // ⚠️ If you add a new plugin here, also add it to BevySpritePlugin
   pub mod my_new_plugin; // ← then add MyNewPlugin::new() to BevySpritePlugin::build()
   ```

2. **Resource insertion point convention**: Every `#[derive(Resource)]` struct must have exactly one insertion point in a `Plugin::build()`. If a resource is only used in tests and not in any `Plugin::build()`, it will panic at runtime. Add a test:

   ```rust
   #[test]
   fn test_all_resources_inserted_in_plugin_build() {
       // Verify VoxpartyWorld is insertable via WorldPlugin::build()
       let mut app = App::new();
       app.add_plugins(WorldPlugin::new());
       assert!(app.world().contains_resource::<VoxpartyWorld>());
   }
   ```

3. **Integration smoke test**: A `cargo test` that runs one Bevy frame would catch missing resources and unregistered systems:

   ```rust
   #[test]
   fn test_bevy_app_runs_one_frame() {
       let mut app = App::new();
       app.add_plugins(BevySpritePlugin);
       // Would panic here if VoxpartyWorld is missing
       let world = app.world();
       assert!(world.contains_resource::<VoxpartyWorld>());
   }
   ```

4. **System ordering check**: Any pair of systems that both modify `Transform` must have explicit `.before()`/`.after()` ordering. Document this in the plugin's module docstring.

5. **Parallel agent coordination**: For multi-agent sprints, establish the integration contract (the combined plugin's `build()` method) BEFORE dispatching parallel workers. Each agent's plugin must be explicitly listed in that `build()`.

## Related Issues

- [VoxParty Bevy Migration Plan](docs/plans/2026-03-30-012-refactor-bevy-migration-plan.md) — Phase 3 scope and goals
- [GridPos Type Divergence Between Parallel Agents](docs/solutions/logic-errors/gridpos-type-divergence-parallel-agents-2026-03-30.md) — separate parallel agent issue in same sprint
- [Debug Infrastructure Build Sprint](docs/solutions/2026-03-30-debug-infrastructure-build.md) — Sprint 22 retrospective (multi-sprint coordination)
