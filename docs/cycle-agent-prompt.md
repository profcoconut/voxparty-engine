# VoxParty Autonomous Test Cycle — Agent Prompt

This prompt fires every 30 seconds via `/loop`. It handles the full
investigate → fix → commit → memory-write cycle autonomously.

## The Prompt

```
You are running the VoxParty autonomous test cycle.

WORKING DIRECTORY: /Users/tengpeng/Documents/voxparty/.worktrees/engine/
MEMORY_DIR: /Users/tengpeng/.claude/projects/-Users-tengpeng-Documents-voxparty/memory/
BRANCH: feat/engine
CYCLE_BUDGET: 20 seconds
CYCLE_INTERVAL: 30 seconds

---

0. PRE-FLIGHT CHECK
   - Run `git branch --show-current` and verify you are on `feat/engine`.
   - If not on feat/engine, log "[CYCLE] wrong branch — skipping" and exit.
   - Run `git status --short`. If the repo has uncommitted changes (not from a
     previous cycle WIP), log "[CYCLE] dirty working tree — skipping" and exit.
   - Check for a WIP marker file: if `.cycle-wip` exists and is less than
     2 cycles old (less than 60 seconds), log "[CYCLE] resuming WIP" and
     attempt to resume the in-progress investigation from the marker.
     If the marker is stale (>60s old), remove it and start fresh.
   - Record cycle start time.

1. RUN TESTS
   Run `cargo test 2>&1` and capture all output.

2. PASS — exit fast
   If output contains "test result: ok" with zero failures:
   eprintln!("[CYCLE {}] OK — all tests passing", <timestamp>);
   exit.

3. FAIL — investigate and fix
   a. Parse ALL failing test names and error messages from output.
      Group failures. Before fixing anything, ask: do these share a common
      root cause? If yes, investigate the shared source. If no, pick one.
   b. Read the failing test file and the source file it tests.
   c. Understand the bug from the test assertion + source code.
   d. Write the fix to the source file.
      IMPORTANT: fix the SOURCE code, never the failing test itself.
      If the test itself appears wrong, flag it for human review instead.
   e. Add a regression test for this bug in the appropriate test module.
   f. Run `cargo test 2>&1` to verify green.
   g. If green:
      - git stash         (save any uncommitted human changes)
      - git add <files>
      - git commit -m "fix(<module>): <imperative description>

      Body: Root cause: <what was wrong>
            Fix: <what was changed and why>"
      - git stash drop    (drop the stash if it was from this cycle)
   h. Write a memory entry (see below).
   i. Remove any .cycle-wip marker for this investigation.
   j. Exit.

4. PANIC — handle separately
   If output contains "panicked" or "thread.*panicked":
   a. Parse the panic message and backtrace.
   b. Identify the source file and line number from the backtrace.
   c. Read the relevant source file.
   d. Investigate the cause and write a fix.
   e. Add a regression test.
   f. Run `cargo test` to verify green.
   g. Commit with the same message format as step 3g.
   h. Write a memory entry (see below).
   i. Exit.

5. TIMEOUT
   If >20 seconds have elapsed since step 0:
   - Write a `.cycle-wip` file with:
     * timestamp of when the cycle started
     * what bug was being investigated (failing test name)
     * what source file was being examined
     * any partial fix notes
   - Do NOT commit partial work.
   - Do NOT leave cargo test in a failing state if avoidable.
   - Exit and wait for the next cycle to resume.

---

## Memory Entry Format

After each fix (step 3h or 4h), write a new file to MEMORY_DIR with name:
  `YYYYMMDD-HHMMSS-<slug>.md`

Example filename: `20260329-143022-fps-ema-alpha-too-high.md`

File content:
```markdown
---
name: <slug>
description: <one-line summary of what went wrong>
type: feedback
attempt: 1
outcome: succeeded
---

<Bug description: what went wrong, in 1-3 sentences.>

**Root cause:** <the underlying reason the bug existed>

**Diagnosis:** <how the bug was identified — what clue led to the root cause>

**How to apply:** <what to look for if this bug recurs, and how to verify the fix>
```

- Filename slug: short, lowercase, hyphenated (e.g., `fps-ema-alpha-too-high`)
- `attempt` field: increment each time the same bug recurs (attempt: 1, attempt: 2, ...)
- `outcome` field: `succeeded` | `failed` | `inconclusive`
- Entries are APPEND-ONLY. Never overwrite an existing entry.
  If the same bug recurs, create a new entry with `attempt: N+1`.
  A chain of failed attempts for the same bug signals a human needs to investigate.

---

## Key Principles

- One fix per cycle maximum.
- Fix SOURCE code, never the failing test.
- Never commit without `cargo test` passing.
- If timeout fires, write `.cycle-wip` and exit — do not leave tests red.
- Memory entries are append-only, never overwritten.
```

## Usage

Register with the `/loop` skill:

```
/loop 30s "<paste prompt text from above>"
```

If `/loop` does not support second-level intervals, use CronCreate instead:

```
CronCreate(cron="* * * * *", prompt="...", recurring=true)
```

Note: CronCreate fires every minute. Adjust the cycle budget to 50 seconds
and the WIP staleness threshold to 120 seconds accordingly.
