# Developer Loop — Continuous Self-Improvement

VoxParty uses a continuous background agent loop to catch regressions, investigate failures, and compound debugging knowledge over time.

## Two Modes

This project supports two complementary loop modes:

- **Autonomous mode** (recommended): A scheduled agent runs the cycle automatically with no human intervention. See [Autonomous Mode](#autonomous-mode) below.
- **Human-in-the-loop mode**: A human operator runs the cycle steps manually, using the debug overlay to investigate. See [Human-in-the-Loop Mode](#human-in-the-loop-mode) below.

Both modes write to the same memory system and follow the same TDD discipline.

---

## Autonomous Mode

The autonomous mode runs the full investigate → fix → commit → memory-write cycle automatically, waking every 30 seconds to run `cargo test`.

### Setup

Register the cycle prompt with the `/loop` skill:

```
/loop 30s "<paste contents of docs/cycle-agent-prompt.md>"
```

If `/loop` does not support second-level intervals, use `CronCreate` instead for 1-minute granularity:

```
CronCreate(cron="* * * * *", prompt="<cycle agent prompt>", recurring=true)
```

### How It Works

Each cycle:

0. **Pre-flight**: Verifies `feat/engine` branch, clean working tree, no stale WIP marker
1. **Run tests**: `cargo test` in `.worktrees/engine/`
2. **Pass**: logs "cycle OK" and exits
3. **Fail**: investigates (grouping by root cause), fixes source (never the test), adds regression test, verifies green, commits, writes memory entry
4. **Panic**: parses backtrace, investigates, fixes, adds test, commits, writes memory entry
5. **Timeout** (>20s): writes `.cycle-wip` marker and exits — next cycle resumes

### Key Constraints

- One fix per cycle maximum
- Always fix SOURCE code, never the failing test
- Never commit without `cargo test` passing
- Memory entries are **append-only** (never overwrite — each recurrence gets `attempt: N+1`)
- Cycle budget: 20s. If exceeded, WIP marker is written and next cycle resumes

### WIP Recovery

If the agent times out mid-investigation, it writes a `.cycle-wip` file in the engine root. The next cycle reads this file and resumes rather than starting over.

### Session Restart

When resuming after a session restart:

1. Check for stale `.cycle-wip` files and clean them up
2. Run `cargo test` to establish baseline
3. Proceed with the loop

### Limitations

- Runs only while the Claude Code session is active. If the session dies, the loop stops.
- Cannot investigate visual/rendering bugs (no display access)
- Cannot catch runtime panics that only occur outside unit tests
- For 24/7 operation, a host-level watchdog process (systemd/cron) is needed

---

## Human-in-the-Loop Mode

Run in a long-lived Claude Code session. Each cycle:

1. **Run tests** — `cargo test` in `.worktrees/engine/`
2. **If tests fail:** read the failing test and source, use the debug overlay to understand game state, fix the bug, add a regression test, verify green
3. **If tests pass:** wait 60 seconds, repeat
4. **If a panic occurs:** parse the panic message and backtrace, identify source file and line, investigate, fix, add test
5. **After any fix:** write a memory entry capturing the root cause and diagnosis method

When you are pointed at a visual issue ("fix this"), activate the debug overlay (F1), identify the relevant internal state (camera position, grid coordinates, depth key), trace to the source, fix, and add a test.

## Debug Overlay Cheat Keys

Press **F1** to toggle the debug overlay. When active, additional keys are enabled:

| Key | Effect |
|-----|--------|
| F1 | Toggle debug overlay |
| R | Reload sprite assets from disk |
| G | Toggle god mode (player immune to traps) |
| 1 | Jump to Menu scene |
| 2 | Jump to TitleCard scene |
| 3 | Jump to Playing scene |
| 4 | Jump to GameOver scene |
| C | Print camera state to stderr |
| P | Print player state to stderr |

## Memory Format

After diagnosing a bug, write a memory entry to:

```
/Users/tengpeng/.claude/projects/-Users-tengpeng-Documents-voxparty/memory/
```

Entries are **append-only with timestamp filenames**. Never overwrite an existing entry. Each recurrence of the same bug gets a new file with `attempt: N+1`.

### Feedback entry format

```markdown
---
name: <slug>
description: <one-line summary>
type: feedback
attempt: 1
outcome: succeeded
---

<Bug description>

**Root cause:** <the underlying reason>

**Diagnosis:** <how the bug was identified>

**How to apply:** <what to look for if this recurs>
```

The `outcome` field is `succeeded`, `failed`, or `inconclusive`. A chain of failed attempts for the same bug name signals a human needs to investigate.

### Reference entry format

```markdown
---
name: <slug>
description: <one-line summary>
type: reference
attempt: 1
outcome: succeeded
---

<What to know> **Why:** <context> **How to apply:** <where to look>
```

## Session Startup

When resuming a session:

1. Check the memory directory for recent feedback entries
2. Run `cargo test` to establish baseline
3. Check for stale `.cycle-wip` files from a previous crashed cycle — clean if present
4. If tests were failing, investigate before starting new work

## TDD Enforcement

Per CLAUDE.md: **write failing tests before implementing.** Every failing test must be investigated and resolved — either fix the implementation or genuinely fix the test itself.

## Interval Tuning (Human-in-the-Loop)

The default sleep between test cycles is 60 seconds. If `cargo test` is slow (>10s), increase to 120s. If fast (<3s), decrease to 30s.
