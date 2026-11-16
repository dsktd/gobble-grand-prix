mesa := `nix build --no-link --print-out-paths nixpkgs#mesa 2>/dev/null || true`

# Build the optimised game
build:
    nix develop -c nice -n 19 cargo build --release -j8

# Play
run: build
    nix develop -c ./target/release/super-slop-kart

# Play with Mesa's RADV driver (inside the Claude container)
run-radv: build
    VK_ICD_FILENAMES={{mesa}}/share/vulkan/icd.d/radeon_icd.x86_64.json nix develop -c ./target/release/super-slop-kart

# Scripted screenshot tour of one track (0, 1 or 2) into screenshots/
shots track="0": build
    VK_ICD_FILENAMES={{mesa}}/share/vulkan/icd.d/radeon_icd.x86_64.json nix develop -c ./target/release/super-slop-kart shots {{track}}

# Steam Deck test binary (glibc 2.35, Zen 2) in target/deck/x86_64-unknown-linux-gnu/deck-dev/; add --release for tags
deck-build *args:
    ./deck-build.sh {{args}}

# Exactly what CI's lint job runs 
lint:
    nix develop -c cargo fmt --all --check
    nix develop -c cargo clippy --all-targets --all-features --locked -- -D warnings

# Headless layout tests: no text may overflow on any screen or HUD
test:
    nix develop -c cargo test --locked --lib
