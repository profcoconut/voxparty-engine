#!/bin/bash
set -e
echo "=== Visual Tests ==="
bash tests/visual_test.sh
echo "=== Unit Tests ==="
cargo test