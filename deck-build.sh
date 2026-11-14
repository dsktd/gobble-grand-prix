#!/usr/bin/env bash
# Release build for the Steam Deck: glibc 2.35 floor via zig, Zen 2 codegen.
set -euo pipefail
cd "$(dirname "$0")"
export RUSTFLAGS="-C target-cpu=znver2"
exec nice -n 19 nix develop --command nix shell nixpkgs#cargo-zigbuild nixpkgs#zig -c \
  cargo zigbuild --release -j8 --target x86_64-unknown-linux-gnu.2.35
