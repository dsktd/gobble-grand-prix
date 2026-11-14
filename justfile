mesa := `nix build --no-link --print-out-paths nixpkgs#mesa 2>/dev/null || true`

# Build the optimised game
build:
    nix develop -c nice -n 19 cargo build --release -j8

# Play
run: build
    nix develop -c ./target/release/gobble

# Play with Mesa's RADV driver (inside the Claude container)
run-radv: build
    VK_ICD_FILENAMES={{mesa}}/share/vulkan/icd.d/radeon_icd.x86_64.json nix develop -c ./target/release/gobble

# Scripted screenshot tour of one track (0, 1 or 2) into screenshots/
shots track="0": build
    VK_ICD_FILENAMES={{mesa}}/share/vulkan/icd.d/radeon_icd.x86_64.json nix develop -c ./target/release/gobble shots {{track}}

# Steam Deck binary (glibc 2.35, Zen 2) in target/x86_64-unknown-linux-gnu/release/
deck-build:
    ./deck-build.sh
