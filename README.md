# Gobble Grand Prix

A small Thanksgiving kart racer in Rust and Bevy. Every mesh and sound is made in code.

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

- Racers: Tom Gobbler (turkey in a gravy boat), Patty Crust (pumpkin pie), Cranston Can (cranberry sauce), Kernel Cob (corn), Elder Spud (pilgrim potato), Marsha Mallow (sweet potato casserole).
- Tracks: Harvest Lane (farm road at sunset), Grand Feast Table (around the roast, with a gravy-river shortcut), Maize Maze (cornfield at dusk).
- Items from harvest crates: Cranberry (homing), Gravy Slick (dropped behind), Stuffing Boost.
- 3 laps, 5 CPU racers with rubber banding, countdown, results board, options (volume, quality, controls).

`gobble shots <track>` runs a scripted tour and saves screenshots to `screenshots/`.

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
`deck/shortcut.py` (run on the Deck as `deck`, with Steam stopped) adds the "Gobble Grand Prix" non-Steam shortcut
with that art. On the Deck the game runs borderless fullscreen at 1280x800.

## Music

The game plays its own synthesised harvest-fair soundtrack (menu loop, race loop, faster final lap). To use your
own music, put files in `~/.local/share/gobble-gp/music/` (`$XDG_DATA_HOME/gobble-gp/music/`):

- `menu.ogg` / `menu.mp3` / `menu.flac` plays on the menus, `race.*` during races;
- with neither name present, every other file there becomes the race playlist.

Options has a separate Music volume. Nothing from that folder is ever part of the game.

## Phone controller

The title screen shows a QR code. Scan it with a phone, tap "Tap to start", then press A to join a seat.

- **Any network (default):** the QR opens `https://dsktd.github.io/gobble-grand-prix/pad/#g=<game id>&pin=<PIN>`.
  The page runs iroh in WebAssembly and reaches the game through iroh's public relays, so the phone can be on
  mobile data. The game id and PIN sit in the URL fragment, so no web server sees them; the id changes every
  time the game starts.
- **Same Wi-Fi:** `https://<LAN IP>:7777/?pin=<PIN>`, shown under the QR. It uses a self-signed certificate:
  accept the warning once.

Both carry the same newline-delimited JSON messages. `pad/` holds the browser module and `pad-probe`, a
native test client (`pad/build.sh`, then `pad/target/release/pad-probe <game id> <PIN>`); `pad/publish.sh`
updates the GitHub Pages copy.
