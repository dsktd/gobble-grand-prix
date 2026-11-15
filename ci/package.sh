#!/usr/bin/env bash
# package.sh <binary> <archive-name> <tar|zip>: bundle binary, assets and licences.
set -euo pipefail
bin=$1 name=$2 kind=$3
dir=pkg/super-slop-kart
rm -rf pkg && mkdir -p "$dir"
cp "$bin" "$dir/"
cp -R assets "$dir/assets"
cp README.md UNLICENSE CREDITS.md "$dir/"
# Third-party licences travel with the assets they cover.
for l in assets/*/License* assets/*/LICENSE* assets/*/OFL*; do
  [ -f "$l" ] && cp "$l" "$dir/$(basename "$(dirname "$l")")-$(basename "$l")"
done
if [ "$kind" = zip ]; then (cd pkg && 7z a -tzip "../$name" super-slop-kart >/dev/null || zip -qr "../$name" super-slop-kart)
else tar czf "$name" -C pkg super-slop-kart; fi
