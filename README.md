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

## License

Public domain, see UNLICENSE.
