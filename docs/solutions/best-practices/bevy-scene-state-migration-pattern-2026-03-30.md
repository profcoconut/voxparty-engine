---
title: Bevy Scene State Migration Pattern
date: 2026-03-30
problem_type: best_practice
module: bevy-plugins
component: tooling
severity: medium
applies_when:
  - Migrating custom game-state enums to Bevy State API
  - Replacing match-dispatch control flow with OnEnter/OnExit per-state systems
  - Converting manual f32 timer fields to Bevy Timer Resources
  - Migrating state transitions scattered across multiple call sites to NextState<GameState>
tags:
  - bevy
  - state-migration
  - game-engine
  - bevy-state
  - scene-management
related_components:
  - bevy_plugins/game_state.rs
  - core/scene.rs
  - lib.rs
---

# Bevy Scene State Migration Pattern

## Context

Phase 6 of the VoxParty Bevy migration required replacing a hand-rolled state machine with Bevy's State API. The codebase had:

- A `SceneState` enum with 7 variants: `Menu`, `EpisodeSelect`, `TitleCard`, `Playing`, `Paused`, `Victory`, `GameOver`
- A `Scene` struct holding `state: SceneState`, `prev_state: SceneState`, and `just_entered_*()` helpers
- A 700-line `match scene.state` block in `lib.rs` dispatching all rendering
- Raw `f32` timer fields (`title_timer`, `gameover_timer`, `victory_timer`) decremented manually each frame
- 20+ transition call sites scattered across `lib.rs` (keyboard input, gamepad polling, game logic, timer expiry)

The pattern relied on opaque manual tracking. Bevy's scheduler — its ability to run specific systems only in specific states — was invisible to this code.

## Guidance

### Step 1: Use an Existing State Enum

If a `GameState` enum with `#[derive(States)]` already exists and mirrors the target state space, reuse it. Do not create a parallel enum. The existing enum in `src/bevy_plugins/game_state.rs` already had the right variants.

```rust
// Already existed — use it
#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub enum GameState {
    #[default]
    Menu,
    EpisodeSelect,
    TitleCard,
    Playing,
    Paused,
    Victory,
    GameOver,
}
```

### Step 2: Slim the Entity Struct to a Resource

Remove `state`, `prev_state`, and `just_entered_*()` helpers from the entity struct. Derive `Resource`. Keep only per-state data — timers, flags, save data.

```rust
// Before: Scene held state fields
struct Scene {
    state: SceneState,
    prev_state: SceneState,
    title_timer: f32,
    gameover_timer: f32,
    victory_timer: f32,
    // ... many other fields
    fn just_entered_playing(&self) -> bool {
        self.state == SceneState::Playing && self.prev_state != SceneState::Playing
    }
}

// After: SceneData holds only per-state data
#[derive(Resource)]
struct SceneData {
    title_timer: f32,
    gameover_timer: f32,
    victory_timer: f32,
    // titlecard_fade_timer, titlecard_pulse_timer (f32 for animation)
    save_data: SaveData,
    selected_episode_index: usize,
    // flags: player_died_this_run, checkpoint_hit_this_run, npc_seen_this_episode
}
```

Register in the plugin's `build()`:

```rust
app.init_resource::<SceneData>();
```

### Step 3: Convert Timers to Bevy Timer Resources

Replace raw `f32` fields with `bevy::time::Timer`. Use `TimerMode::Once` for auto-transitions (TitleCard → Playing, Victory → Menu, GameOver → Menu). Register timer resources in the plugin, initialize in `OnEnter`, tick in `OnUpdate`.

```rust
#[derive(Resource)]
pub struct TitleCardTimer(pub Timer);

#[derive(Resource)]
pub struct GameOverTimer(pub Timer);

#[derive(Resource)]
pub struct VictoryTimer(pub Timer);

// In Plugin::build():
app.init_resource::<TitleCardTimer>();
app.init_resource::<GameOverTimer>();
app.init_resource::<VictoryTimer>();

app.add_systems(OnEnter(GameState::TitleCard), init_titlecard_timer);
app.add_systems(OnEnter(GameState::GameOver), init_gameover_timer);
app.add_systems(OnEnter(GameState::Victory), init_victory_timer);

fn init_titlecard_timer(mut timer: Query<&mut TitleCardTimer>) {
    if let Ok(mut t) = timer.get_single_mut() {
        t.0 = Timer::from_seconds(3.0, TimerMode::Once);
    }
}
```

Auto-transitions in `OnUpdate` per state:

```rust
app.add_systems(OnUpdate(GameState::TitleCard), titlecard_auto_play);
app.add_systems(OnUpdate(GameState::GameOver), gameover_auto_menu);
app.add_systems(OnUpdate(GameState::Victory), victory_auto_menu);

fn titlecard_auto_play(
    mut timer: Query<&mut TitleCardTimer>,
    time: Res<Time>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Ok(mut t) = timer.get_single_mut() else { return };
    t.0.tick(time.delta());
    if t.0.finished() {
        next.set(GameState::Playing);
    }
}
```

### Step 4: Per-State OnEnter/OnExit Systems

Replace `just_entered_*()` helpers and match dispatch with `OnEnter(GameState::X)` and `OnExit(GameState::X)` schedules. These run automatically when Bevy transitions into or out of a state — no manual tracking needed.

```rust
app.add_systems(OnEnter(GameState::Menu), menu_enter_system);
app.add_systems(OnEnter(GameState::EpisodeSelect), episode_select_enter_system);
app.add_systems(OnEnter(GameState::TitleCard), (reload_episode_game_state, titlecard_enter_system).chain());
app.add_systems(OnEnter(GameState::Playing), playing_enter_system);
app.add_systems(OnEnter(GameState::Victory), victory_enter_system);
app.add_systems(OnEnter(GameState::GameOver), gameover_enter_system);
app.add_systems(OnEnter(GameState::Paused), paused_enter_system);

app.add_systems(OnExit(GameState::Menu), menu_exit_system);
app.add_systems(OnExit(GameState::Playing), playing_exit_system);
// ... exit systems as needed for cleanup
```

`OnEnter` systems handle: UI text setup, camera positioning, telemetry session start, HealthMonitor reset, timer initialization.

`OnExit` systems handle: cleanup, telemetry session end, particle flush.

### Step 5: Wire Transition Call Sites to NextState

Replace all `scene.state = X` mutations and transition method calls with `NextState<GameState>`. Each transition becomes a system that accepts `ResMut<NextState<GameState>>`.

```rust
// Before (scattered across lib.rs):
scene.start_episode_select();
scene.trigger_victory(Some(1));
scene.return_to_menu();

// After: Transition systems
fn go_to_episode_select(mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::EpisodeSelect);
}

fn go_to_titlecard(mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::TitleCard);
}

fn trigger_victory_system(
    mut next: ResMut<NextState<GameState>>,
    players: Query<&PlayerComponent>,
) {
    for player in &players {
        if player.state == PlayerState::Won {
            next.set(GameState::Victory);
            return;
        }
    }
}
```

Use `.chain()` to order systems that must run before others:

```rust
// reload_episode_game_state must run BEFORE titlecard_enter_system
app.add_systems(OnEnter(GameState::TitleCard), (
    reload_episode_game_state,
    titlecard_enter_system,
).chain());
```

### Step 6: Replace State Checks with in_state() Guards

Replace `scene.state == SceneState::X` checks with `in_state(GameState::X)` system conditionals.

```rust
// Before:
if scene.state == SceneState::Playing {
    render_hud(&plat, &player);
}

// After:
app.add_systems(Update, hud_system.run_if(in_state(GameState::Playing)));
app.add_systems(Update, debug_overlay_system.run_if(in_state(GameState::Playing)));
```

This applies to: HUD rendering, minimap, debug overlay, telemetry emission, console toggle, HealthMonitor assertions.

## Why This Matters

- **Declarative over imperative**: State transitions are no longer mutations scattered across 20 call sites. They are centralized, observable events in the Bevy schedule.
- **Bevy schedule control**: `OnEnter`/`OnExit` schedules fire automatically. No `just_entered_*()` bookkeeping needed.
- **Timer auto-advance**: `Timer::tick(time.delta())` replaces manual `timer -= delta_time; if timer <= 0.0`. Bevy handles the arithmetic correctly, including pausing when the schedule is not active.
- **Schedule isolation**: Systems with `run_if(in_state(...))` only execute in the right state. No manual `match` dispatch needed.
- **Telemetry-friendly**: `OnEnter(Playing)` naturally calls `telemetry.session_start()`. The transition is the trigger, not an ad-hoc flag check.

## When to Apply

- **Phase N Bevy migrations**: When migrating any manual state machine to Bevy (confirmed pattern across Phases 2-6: sprites → ECS → audio → state).
- **Pre-existing State enum**: If the codebase already has a `#[derive(States)]` enum, migrate to it rather than creating new types.
- **Greenfield Bevy game**: Start with `app.init_state::<GameState>()` from day one. All state-dependent systems get `run_if(in_state(...))`.
- **Bevy 0.18+**: This pattern uses `States`, `init_state`, `OnEnter`, `OnExit`, `in_state`, `NextState`, and `Timer` from `bevy::time`.

## Examples

### Before: 700-line match dispatch

```rust
// lib.rs — dispatch by matching scene.state
match scene.state {
    SceneState::Menu => {
        debug_overlay.draw_text(&mut plat, "VOXPARTY", ...);
        debug_overlay.draw_text(&mut plat, "PRESS SPACE TO START", ...);
    }
    SceneState::EpisodeSelect => {
        // 150 lines of episode list rendering
    }
    SceneState::Playing => {
        render_tiles(&world, &mut plat);
        render_npcs(&npcs, &mut plat);
        render_players(&players, &mut plat);
        render_hud(&plat, &player);
    }
    SceneState::Paused => {
        plat.clear(...);
        render_tiles(&world, &mut plat);
        debug_overlay.draw_text(&mut plat, "PAUSED", ...);
    }
    // ... 500 more lines
}
```

### After: State-driven rendering

```rust
// lib.rs — minimal, driven by in_state()
app.add_systems(Update, (
    hud_system.run_if(in_state(GameState::Playing)),
    minimap_system.run_if(in_state(GameState::Playing)),
    debug_overlay_system.run_if(in_state(GameState::Playing)),
));

// bevy_plugins/scene_plugin.rs — clean per-state setup
app.add_systems(OnEnter(GameState::Menu), (
    camera_reset_system,
    menu_text_system,
));
app.add_systems(OnEnter(GameState::Playing), (
    camera_center_on_spawn_system,
    health_reset_system,
    telemetry_session_start_system,
));
app.add_systems(OnEnter(GameState::Paused), paused_overlay_system);
```

### Console Toggle Migration

```rust
// Before:
if scene.state == SceneState::Playing && console_toggled {
    console.render(...);
}

// After:
app.add_systems(Update, (
    console_input_system,
    console_render_system.run_if(console_visible),
));
// console_visible is a bool Resource toggled by console_input_system
// State gating is handled by which systems are registered, not by matching
```

## Prevention

1. **Define GameState enum before plugin work starts**: An enum with `#[derive(States)]` must exist and be registered via `app.init_state::<GameState>()` before any `OnEnter`/`OnExit` systems are added. Add it in the first migration sprint.

2. **One plugin owns state transitions**: Centralize all transition logic in one plugin (`ScenePlugin`). Other plugins trigger transitions via `NextState<GameState>` — they do not call `SceneData` mutation methods.

3. **Timer initialization in OnEnter, not OnUpdate**: Timers must be initialized in `OnEnter` and ticked in `OnUpdate`. If a timer is initialized in `OnUpdate`, it resets every frame. If it is ticked in `OnEnter`, it ticks once.

4. **Test the state machine in isolation**: A smoke test that transitions through all states (Menu → EpisodeSelect → TitleCard → Playing → Victory → Menu) catches integration gaps without rendering. Add this as a headless test:

   ```rust
   #[test]
   fn test_game_state_transitions() {
       let mut app = App::new();
       app.add_plugins(BevyVoxpartyPlugin);
       let world = app.world();

       // Verify initial state
       let state = world.resource::<State<GameState>>();
       assert_eq!(state.get(), &GameState::Menu);

       // Manually drive transitions via NextState
       // ... verify each state is reachable
   }
   ```

5. **Register new timer resources in Plugin::build()**: Every `#[derive(Resource)]` struct must be initialized with `app.init_resource::<X>()` in a `Plugin::build()`. If a resource is missing, Bevy panics on first query.

## Related

- [Bevy Non-Send Resource FFI Pattern](docs/solutions/best-practices/bevy-nonsend-resource-ffi-haptic-2026-03-30.md) — Sister pattern: wrapping FFI handles as non-send resources in the same plugin series
- [Bevy Phase 3 Plugin Wiring](docs/solutions/integration-issues/bevy-phase3-plugin-wiring-2026-03-30.md) — Predecessor sprint: plugin composition root, resource insertion points, system ordering
- [Debug Infrastructure Build](docs/solutions/2026-03-30-debug-infrastructure-build.md) — Sprouts 15-22: HealthMonitor, TelemetryLogger, and the `run_if(in_state(...))` guard pattern that preceded this migration
- `src/bevy_plugins/game_state.rs` — GameState enum
- `src/core/scene.rs` — SceneData (post-migration)
- `src/bevy_plugins/scene_plugin.rs` — ScenePlugin with OnEnter/OnExit systems
