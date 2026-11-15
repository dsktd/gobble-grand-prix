#!/usr/bin/env bash
# Lay out the phone pad page in <dir>/pad/: the page, its font, the Kenney glyphs it uses (the
# same files src/phone.rs serves on the LAN) and the iroh wasm module from pad/pkg.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
out=$1/pad
mkdir -p "$out/pkg" "$out/fonts" "$out/glyphs"
cp "$root/assets/phone/index.html" "$out/"
cp "$root"/assets/fonts/Fredoka-{500,600,700}.ttf "$root/assets/fonts/OFL.txt" "$out/fonts/"
for glyph in $(grep -o 'glyphs/[a-z0-9_]*\.png' "$root/assets/phone/index.html" | sort -u); do
  cp "$root/assets/prompts/steamdeck/${glyph#glyphs/}" "$out/glyphs/"
done
cp "$root/assets/prompts/License.txt" "$out/glyphs/"
cp "$root"/pad/pkg/slop_pad.js "$root"/pad/pkg/slop_pad_bg.wasm "$out/pkg/"
