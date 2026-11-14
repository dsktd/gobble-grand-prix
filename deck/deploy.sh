#!/usr/bin/env bash
# Build and stage the game on the Steam Deck with progress in the Deck overlay (deckjob):
# build, copy, then a swap that checks the hash, moves the new binary in atomically (a running
# game keeps its copy, the next launch gets the new one), checks ldd and writes BUILD.txt.
# Never launches or kills the game.
#   deck/deploy.sh [--release]
set -euo pipefail
cd "$(dirname "$0")/.."
D=~/claude/.services/deckupdate/deckjob
dk=~/claude/nx2navi/tools/deck/dk
profile=deck-dev
[ "${1:-}" = "--release" ] && profile=release
bin=target/deck/x86_64-unknown-linux-gnu/$profile/gobble
commit=$(git describe --always --dirty)

stage() {
  "$D" cargo "Gobble GP" -- ./deck-build.sh "$@" || return 1
  local sum built
  sum=$(sha256sum "$bin" | cut -d' ' -f1)
  built=$(date -u -r "$bin" +%FT%TZ)
  "$D" copy "Gobble GP" "$bin" deck:/tmp/gobble.new || return 1
  "$D" run "Gobble GP" swap -- "$dk" "
    set -e
    cd ~/Games/gobble-gp
    install -m755 /tmp/gobble.new gobble.new
    [ \"\$(sha256sum gobble.new | cut -d' ' -f1)\" = '$sum' ]
    ! ldd gobble.new | grep 'not found'
    mv gobble.new gobble
    printf 'commit %s\nprofile %s\nbuilt %s\nsha256 %s\n' '$commit' '$profile' '$built' '${sum:0:16}' > BUILD.txt
    cat BUILD.txt" || return 1
}

if stage "$@"; then
  "$D" "done"
else
  "$D" fail
  exit 1
fi
