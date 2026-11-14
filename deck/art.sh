#!/usr/bin/env bash
# Steam library art from the screenshot tour: grid (600x900, 920x430), hero, logo, icon into deck/art/.
set -euo pipefail
cd "$(dirname "$0")"
shots=../screenshots
mkdir -p art
font=$(nix build --no-link --print-out-paths nixpkgs#dejavu_fonts)/share/fonts/truetype/DejaVuSans-Bold.ttf
nix shell nixpkgs#imagemagick -c bash -s "$shots" "$font" <<'SH'
set -euo pipefail
shots=$1 font=$2
title=(-font "$font" -fill "#ffd25a" -stroke "#5a2008" -strokewidth 3)
magick "$shots/harvest-lane-racing.png" -crop 900x1250+187+182 +repage -resize 600x900^ -gravity center -extent 600x900 \
  "${title[@]}" -gravity north -pointsize 66 -annotate +0+40 "GOBBLE\nGRAND PRIX" art/portrait.png
magick "$shots/grand-feast-table-racing.png" -crop 1274x1250+0+182 +repage -resize 920x430^ -gravity center -extent 920x430 \
  "${title[@]}" -gravity northwest -pointsize 54 -annotate +30+24 "GOBBLE GRAND PRIX" art/wide.png
magick "$shots/maize-maze-racing.png" -crop 1274x1250+0+182 +repage -resize 1920x620^ -gravity center -extent 1920x620 art/hero.png
magick -size 1200x300 xc:none "${title[@]}" -strokewidth 6 -gravity center -pointsize 130 \
  -annotate +0+0 "GOBBLE GRAND PRIX" art/logo.png
magick "$shots/menu.png" -crop 560x560+20+230 -resize 256x256 art/icon.png
SH
ls art
