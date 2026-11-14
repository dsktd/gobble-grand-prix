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
magick "$shots/harvest-lane-countdown.png" -crop 390x585+440+85 +repage -resize 600x900^ -gravity center -extent 600x900 \
  "${title[@]}" -gravity north -pointsize 60 -annotate +0+40 "GOBBLE\nGRAND PRIX" art/portrait.png
magick "$shots/harvest-lane-countdown.png" -crop 1000x467+135+130 +repage -resize 920x430^ -gravity center -extent 920x430 \
  "${title[@]}" -gravity northwest -pointsize 54 -annotate +30+24 "GOBBLE GRAND PRIX" art/wide.png
magick "$shots/harvest-lane-countdown.png" -crop 1000x323+135+222 +repage -resize 1920x620^ -gravity center -extent 1920x620 art/hero.png  # no text: Steam overlays the logo
magick -size 1200x340 xc:none "${title[@]}" -strokewidth 6 -gravity center -pointsize 100 \
  -annotate +0+0 "GOBBLE\nGRAND PRIX" art/logo.png
magick "$shots/menu.png" -crop 560x560+20+230 -resize 256x256 art/icon.png
SH
# Steam logo placement: bottom-left, 40% of width
cat > art/logo.json <<'J'
{"nVersion":1,"logoPosition":{"pinnedPosition":"BottomLeft","nWidthPct":40,"nHeightPct":50}}
J
ls art
