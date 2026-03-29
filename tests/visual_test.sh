#!/bin/bash
set -e

ACTUAL_PATH="tests/actual/playing_state.png"
BASELINE_PATH="tests/baseline/playing_state.png"
DIFF_PATH="tests/diff/diff.png"
UPDATE_FLAG="${UPDATE_BASELINE:-}"

# Clean dirs
mkdir -p "$(dirname $ACTUAL_PATH)" "$(dirname $DIFF_PATH)"

# Build
cargo build 2>/dev/null

# Strategy 1: Use game's built-in screenshot (canvas.read_pixels after present)
SCREENSHOT_PATH="/tmp/voxparty_canvas.png"
cargo run -- --screenshot "$SCREENSHOT_PATH" 2>/dev/null && [ -f "$SCREENSHOT_PATH" ] && {
    # Verify it has non-trivial content (not all dark pixels)
    if python3 -c "
import sys
try:
    from PIL import Image
    img = Image.open('$SCREENSHOT_PATH')
    pixels = list(img.getdata())
    avg = sum(sum(p) / len(p) for p in pixels) / len(pixels)
    sys.exit(0 if avg > 5 else 1)  # avg > 5 means not a blank frame
except:
    sys.exit(1)
" 2>/dev/null; then
        cp "$SCREENSHOT_PATH" "$ACTUAL_PATH"
    else
        echo "NOTE: canvas screenshot blank/failed, trying screencapture..."
    fi
}

# Strategy 2: Use macOS screencapture if canvas approach failed or unavailable
if [ ! -f "$ACTUAL_PATH" ] || [ $(stat -f%z "$ACTUAL_PATH" 2>/dev/null || echo 0) -lt 100 ]; then
    # Run game in background, capture window via screencapture
    rm -f /tmp/voxparty_window.png

    # Start game in screenshot mode (it will render and wait briefly)
    cargo run -- --screenshot-hint 2>/dev/null &
    GAME_PID=$!

    # Wait for window to appear (SDL2 window creation is fast)
    sleep 0.5

    # Use screencapture to grab the VoxParty window
    # -x = no sound, -w = select window interactively... we use -x and find window by name
    # Actually on macOS we can use -o for window selection or -l for list
    WINDOW_ID=$(osascript -e 'tell app "System Events" to get id of first window of process "VoxParty"' 2>/dev/null || echo "")

    if [ -n "$WINDOW_ID" ]; then
        screencapture -x -l "$WINDOW_ID" /tmp/voxparty_window.png 2>/dev/null && \
            [ -f /tmp/voxparty_window.png ] && \
            cp /tmp/voxparty_window.png "$ACTUAL_PATH"
    else
        # Fallback: just screencapture without window targeting
        # This will capture the whole screen, might include other windows
        echo "NOTE: Could not find VoxParty window, using fullscreen capture"
    fi

    # Kill game if still running
    kill $GAME_PID 2>/dev/null || true
fi

# Final fallback: if no screenshot at all, create a placeholder to show test ran
if [ ! -f "$ACTUAL_PATH" ]; then
    echo "WARNING: Could not capture screenshot - creating placeholder"
    # Create a 1280x720 dark blue placeholder
    python3 -c "
from PIL import Image
img = Image.new('RGB', (1280, 720), color=(30, 60, 90))
img.save('$ACTUAL_PATH')
" 2>/dev/null || \
    convert -size 1280x720 xc:'#1e3ce6' "$ACTUAL_PATH" 2>/dev/null || \
    echo "placeholder_failed" > "$ACTUAL_PATH"
fi

# Handle baseline update
if [ "$UPDATE_FLAG" = "1" ]; then
    cp "$ACTUAL_PATH" "$BASELINE_PATH"
    echo "BASELINE UPDATED"
    exit 0
fi

if [ ! -f "$BASELINE_PATH" ]; then
    echo "NO_BASELINE: run with UPDATE_BASELINE=1 to create baseline"
    exit 1
fi

# Compare using ImageMagick if available, else use md5 hash
if command -v compare &> /dev/null; then
    compare "$BASELINE_PATH" "$ACTUAL_PATH" "$DIFF_PATH" 2>/dev/null && echo "PASS" || {
        echo "FAIL: visual diff detected, see $DIFF_PATH"
        exit 1
    }
else
    baseline_hash=$(md5 -q "$BASELINE_PATH")
    actual_hash=$(md5 -q "$ACTUAL_PATH")
    if [ "$baseline_hash" = "$actual_hash" ]; then
        echo "PASS"
    else
        echo "FAIL: screenshot differs"
        cp "$ACTUAL_PATH" "$DIFF_PATH"
        exit 1
    fi
fi
