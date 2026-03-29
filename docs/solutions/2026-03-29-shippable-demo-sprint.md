# Sprint 3: shippable-demo — CE Compound

## What We Shipped

Sprint 3 delivered a "genuinely impressive, playable VoxParty demo" with these features:

- **Episode selection** — Menu scans `assets/episodes/` and lets players pick which episode to play
- **Best time tracking** — Per-episode fastest clear times persisted to `assets/saves/best_times.json`
- **Tutorial overlay** — First-launch intro explaining controls (arrows, E talk, ESC pause)
- **Save/Load system** — Persists last episode, best times, and tutorial-shown flag across sessions
- **Third episode** — Night/ice themed `episode-3.json` with distinct tile naming
- **Mobile performance tuning** — FPS profiling and optimizations targeting 30+ FPS on mid-range devices
- **HUD best time display** — Running timer shown during gameplay in MM:SS format
- **Per-episode chiptune soundtrack** — Episode-specific synthesized music loops with pause/victory transitions

## Key Technical Decisions

- **Serde JSON for saves** — Same pattern as episode loading, minimal new code
- **Flag-based tutorial** — Boolean in save file, not a separate "seen tutorials" data structure
- **Episode scanning** — `std::fs::read_dir` at startup, parse only the `metadata` section of each JSON to get title/theme

## What Broke / Lessons Learned

- **20 test failures** — Test episode JSON fixtures (inline or in `tests/`) were not updated when `difficulty` field was added to the episode schema. All fixtures need `difficulty: easy/medium/hard` retroactively.
- **Episode scan on mobile** — `read_dir` may be slow on mobile; consider scanning at compile time or caching episode list.

## Status

- Status: **completed** (2026-03-29T23:30:00Z)
- 8/8 backlog items delivered
- 20 test failures remain — fix-tests-1 is sprint 4 P0
