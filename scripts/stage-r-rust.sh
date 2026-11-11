#!/usr/bin/env sh
set -eu

package="${1:?usage: scripts/stage-r-rust.sh <R package folder> [core folder]}"
core="${2:-$(cd "$(dirname "$0")/.." && pwd)/core}"
rust="$package/src/rust"

rm -rf "$rust/src" "$rust/Cargo.toml" "$rust/Cargo.lock"
mkdir -p "$rust"
cp -R "$core/src" "$rust/src"
cp "$core/Cargo.lock" "$rust/Cargo.lock"

awk '
  /^\[build-dependencies\]/ { skip = 1; next }
  /^\[/ { skip = 0 }
  skip { next }
  /^crate-type/ { print "crate-type = [\"staticlib\"]"; next }
  /^ureq / { next }
  /^http = / { print "http = []"; next }
  { print }
' "$core/Cargo.toml" > "$rust/Cargo.toml"

echo "staged the Rust core into $rust"
