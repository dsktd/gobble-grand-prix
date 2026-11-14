#!/usr/bin/env bash
# Install the newest Deck build on the Steam Deck: atomic swap (a running game keeps its copy,
# the next launch gets the new one) plus BUILD.txt saying what is installed.
#   deck/deploy.sh [deck-dev|release]
set -euo pipefail
cd "$(dirname "$0")/.."
profile=${1:-deck-dev}
bin=target/deck/x86_64-unknown-linux-gnu/$profile/gobble
tools=~/claude/nx2navi/tools/deck
~/claude/.services/locks/games-running deck || true
commit=$(git describe --always --dirty)
"$tools/putbin" "$bin" /home/deck/Games/gobble-gp/gobble
sum=$(sha256sum "$bin" | cut -c1-16)
"$tools/dk" "printf 'commit %s\nprofile %s\nbuilt %s\nsha256 %s\n' '$commit' '$profile' '$(date -u -r "$bin" +%FT%TZ)' '$sum' > ~/Games/gobble-gp/BUILD.txt; cat ~/Games/gobble-gp/BUILD.txt"
