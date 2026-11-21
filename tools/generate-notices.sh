#!/bin/sh
# Regenerates crates/brickpress-app/assets/THIRD_PARTY_NOTICES.txt (bundled in the app).
# Requires: cargo install cargo-about --version 0.9.2 --locked --features cli
set -e
cd "$(dirname "$0")/.."
out=crates/brickpress-app/assets/THIRD_PARTY_NOTICES.txt
cargo about generate --locked --manifest-path crates/brickpress-app/Cargo.toml -c tools/about.toml -o "$out" tools/licenses.hbs
for font in inter ibm-plex-mono lucide; do
  printf '\n\nFont licence: %s\n========================================================================\n' "$font" >> "$out"
  cat "crates/brickpress-app/assets/fonts/LICENSE-$font.txt" >> "$out"
done
echo "Wrote $out"
