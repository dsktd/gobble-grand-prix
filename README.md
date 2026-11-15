# Super Slop Kart

A small pool-hall kart racer, for some reason, in Rust and Bevy. Every mesh and sound is made in code.

## Play

    just run

(`just run-radv` forces Mesa's RADV Vulkan driver.) Needs Nix with flakes; the dev shell brings Rust and the system libraries.

## Controls

| Action    | Keyboard          | Gamepad (Xbox layout) |
| --------- | ----------------- | --------------------- |
| Gas       | W / Up            | A or RT               |
| Brake     | S / Down          | X or LT               |
| Steer     | A D / Left Right  | Left stick or D-pad   |
| Drift     | Space (hold)      | RB or LB (hold)       |
| Item      | Shift or E        | B or Y                |
| Quit race | Esc               | View                  |
| Menus     | Arrows, Enter, Esc | D-pad, A, B          |
| Screenshot | F12              |                       |

Hold drift while steering into a turn: sparks go white, then blue, then orange. Let go for a mini-turbo.

## What's in it

- Racers: Eight Ball Eddie, Stripe Sally, Chalky Blue (a cube of chalk), Rack Rita (a rack of balls), Hustler Hank, Felt Felicia (a pool table) and Wailing Whiskers (a cat).
- Tracks: a new random track every race, in a corner-bar green felt, blue tournament felt or neon pool hall look, with hills, ramps, gaps and the odd figure eight.
- Items from chalk boxes: Cue Ball (homing), Spilled Drink (dropped behind), Cue Kick (boost).
- 3 laps, 5 CPU racers with rubber banding, countdown, results board, options (volume, quality, controls).

`super-slop-kart shots <track> [high|low] [racer] [seats] [top|side|diagonal]` runs a scripted tour and saves screenshots to `screenshots/`.
Set `SLOP_BENCH=<file>` to run the tour without screenshots and write frame-time percentiles to that file (and the log).

## Changelog

Every user-visible change adds one short line at the top of `CHANGELOG.md`. The title screen
shows the newest few under "What's new", and the version line is built in by `build.rs`.

## License

Public domain, see UNLICENSE.

## CI and downloads

CI (`.github/workflows/ci.yml`) runs fmt, clippy and tests. Every push to main that changes the game rebuilds the
"Latest build" pre-release (Steam Deck tarball, generic x86-64-v2 Linux tarball, Windows zip); `v*` tags make
normal releases. `ci/` holds the helper scripts.

## Steam Deck

`just deck-build` makes the Deck binary. `deck/art.sh` turns the screenshot tour into library art, and
`deck/shortcut.py` (run on the Deck as `deck`, with Steam stopped) adds the "Super Slop Kart" non-Steam shortcut
with that art. On the Deck the game runs borderless fullscreen at 1280x800.

## Music

The game ships a CC0 acoustic soundtrack (see `CREDITS.md`): a menu loop and three race loops, one per race in
turn, that speed up a touch on the final lap; cues crossfade. To use your own music, put files in `~/.local/share/super-slop-kart/music/` (`$XDG_DATA_HOME/super-slop-kart/music/`):

- `menu.ogg` / `menu.mp3` / `menu.flac` plays on the menus, `race.*` during races;
- with neither name present, every other file there becomes the race playlist.

Options has a separate Music volume. Nothing from that folder is ever part of the game.

## Phone controller

The title screen shows a QR code. Scan it with a phone, tap "Tap to start", then press A to join a seat.

- **Any network (default):** the QR opens `https://dsktd.github.io/gobble-grand-prix/pad/#g=<game id>&pin=<PIN>&lan=<LAN IP>:7777`.
  The page runs iroh in WebAssembly and reaches the game through iroh's public relays, so the phone can be on
  mobile data. The game id and PIN sit in the URL fragment, so no web server sees them; the id changes every
  time the game starts. If iroh fails or has not connected after 10 s, the page switches on its own to the game's local page below.
- **Same Wi-Fi:** `https://<LAN IP>:7777/?pin=<PIN>`, shown under the QR. It uses a self-signed certificate:
  accept the warning once.

Both carry the same newline-delimited JSON messages. `pad/` holds the browser module and `pad-probe`, a
native test client (`pad/build.sh`, then `pad/target/release/pad-probe <game id> <PIN>`); `pad/publish.sh`
updates the GitHub Pages copy. `pad-host` stands in for the game, and `tools/pad_join_test.py` (CI's Pad
workflow) checks that the page joins it from Chromium and WebKit.
