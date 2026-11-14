#!/usr/bin/env bash
# Headless-ish tuning check on the desktop: run the screenshot tour on each track with every kart
# CPU-driven and print finish times and wall contacts per racer.
#   tools/laps.sh [binary]   (default target/release/gobble)
set -euo pipefail
cd "$(dirname "$0")/.."
bin=${1:-target/release/gobble}
mesa=$(nix build --no-link --print-out-paths nixpkgs#mesa)
export VK_ICD_FILENAMES=$mesa/share/vulkan/icd.d/radeon_icd.x86_64.json
export WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-/run/claude-wayland.d/wayland-0}
for track in 0 1 2; do
  flock ~/claude/.services/locks/gpu.lock timeout 200 nix develop -c "$bin" shots "$track" low 2>&1 \
    | sed 's/\x1b\[[0-9;]*m//g' | grep -o "Finished .*" | sed "s/^/track $track: /" || true
done
