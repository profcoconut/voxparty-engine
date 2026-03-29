# Visual Testing

This directory contains visual regression tests for the VoxParty engine.

## Setup

1. Build the game and create initial baseline:
```bash
UPDATE_BASELINE=1 bash tests/visual_test.sh
```

2. Run visual tests:
```bash
bash tests/visual_test.sh
```

## How it works

1. Game is run with `--screenshot /tmp/voxparty_frame.png`
2. Game renders 1 frame then exits
3. Screenshot is compared against baseline
4. If images differ, test fails and diff is saved to `tests/diff/`

## CI Usage

In GitHub Actions or CI:
```yaml
- name: Visual test
  run: bash tests/visual_test.sh
```

## Notes

- Visual tests require display (SDL2 creates window)
- On macOS CI, use xvfb-run or a display manager
- Screenshot mode bypasses the game loop and renders immediately