#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
here="$root/scripts/check-api"
fixture_dir="$root/core/tests/fixtures/api"
expected="$here/expected.txt"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

update="${UPDATE:-0}"
languages=()
missing=()

run_rust() {
  cargo run --quiet --manifest-path "$root/core/Cargo.toml" --example check_api -- "$fixture_dir" \
    > "$work/rust.txt" 2>"$work/rust.err" || {
    echo "❌ the Rust runner failed:"
    cat "$work/rust.err"
    exit 1
  }
}

run_language() {
  local name="$1" script="$2"
  shift 2
  if [ ! -f "$here/$script" ]; then
    missing+=("$name")
    return
  fi
  if ! "$@" "$here/$script" "$fixture_dir" > "$work/$name.txt" 2>"$work/$name.err"; then
    echo "❌ the $name runner failed:"
    cat "$work/$name.err"
    exit 1
  fi
  languages+=("$name")
}

compare() {
  local name="$1" left="$2" right="$3" label="$4"
  python3 "$here/compare.py" "$left" "$right" "$name" "$label"
}

use_python_with_numpy() {
  if python3 -c 'import numpy' >/dev/null 2>&1; then
    return
  fi
  local venv="$root/.venv-check-api"
  if ! "$venv/bin/python3" -c 'import numpy' >/dev/null 2>&1; then
    echo "python3 has no numpy, installing it in .venv-check-api..."
    python3 -m venv --clear "$venv"
    "$venv/bin/python3" -m pip install --quiet numpy
  fi
  export PATH="$venv/bin:$PATH"
}

echo "check-api — one fixture, nine calls, every language must agree"
echo

if [ ! -f "$fixture_dir/api.ion" ]; then
  echo "❌ fixture missing: $fixture_dir/api.ion"
  echo "   rebuild it from bugs-quantion:"
  echo "   cargo run --bin chromatogram2ion -- $fixture_dir/api.ion 89.04768 data/sarcosine.ion 90.05550 data/alanine.ion"
  exit 1
fi

use_python_with_numpy

echo "running Rust (the reference)..."
run_rust

export QUANTION_ARTIFACTS_ROOT="${QUANTION_ARTIFACTS_ROOT:-$root/artifacts}"
export PYTHONPATH="$root/wrappers/python${PYTHONPATH:+:$PYTHONPATH}"
run_language python run_python.py python3
if command -v Rscript >/dev/null 2>&1 && [ -f "$here/run_r.R" ]; then
  r_lib="$work/rlib"
  mkdir -p "$r_lib"
  if command -v cygpath >/dev/null 2>&1; then
    r_lib="$(cygpath -m "$r_lib")"
  fi
  (cd "$root/wrappers" && tar -cf - --exclude=r/src/rust r) | tar -xf - -C "$work"
  mv "$work/r" "$work/rpkg"
  sh "$root/scripts/stage-r-rust.sh" "$work/rpkg" > /dev/null
  if R CMD INSTALL --no-docs --no-byte-compile -l "$r_lib" "$work/rpkg" > "$work/r_install.log" 2>&1; then
    export R_LIBS="$r_lib"
    run_language r run_r.R Rscript
  else
    echo "the R package failed to install:"
    tail -20 "$work/r_install.log"
    exit 1
  fi
else
  missing+=("r")
fi
js_dir="$root/wrappers/js"
if [ -d "$js_dir/node_modules" ] || ( cd "$js_dir" && npm ci --ignore-scripts >"$work/js_install.log" 2>&1 ); then
  ( cd "$js_dir" && npx node-gyp rebuild && npm run tsc-cjs ) \
    > "$work/js_build.log" 2>&1 || {
    echo "❌ the JS wrapper failed to build:"
    tail -20 "$work/js_build.log"
    exit 1
  }
fi
if [ -f "$js_dir/lib/index-node.js" ]; then
  run_language js run_js.mjs node
else
  missing+=("js")
fi

check_surface() {
  local name="$1"
  shift
  local snapshot="$here/surface/$name.txt"
  if ! "$@" > "$work/surface_$name.txt" 2>"$work/surface_$name.err"; then
    echo "❌ could not read the $name public API:"
    cat "$work/surface_$name.err"
    return 1
  fi
  if ! diff -u --strip-trailing-cr "$snapshot" "$work/surface_$name.txt" > "$work/surface_$name.diff"; then
    echo "❌ the $name public API changed:"
    cat "$work/surface_$name.diff"
    return 1
  fi
  echo "  $name public API unchanged"
}

api_failed=0
echo
echo "checking the public API of every language against scripts/check-api/surface/"
check_surface rust python3 "$here/surface_rust.py" || api_failed=1
check_surface python python3 "$here/surface_python.py" || api_failed=1
if [ -d "$js_dir/node_modules/typescript" ]; then
  check_surface js node "$here/surface_js.mjs" || api_failed=1
else
  missing+=("js public API")
fi
if [ -n "${R_LIBS:-}" ]; then
  check_surface r Rscript "$here/surface_r.R" || api_failed=1
else
  missing+=("r public API")
fi
if [ "$api_failed" -ne 0 ]; then
  echo
  echo "⛔ THE PUBLIC API OF QUANTION CHANGED."
  echo "   The API is frozen. Only the repository owner may change it"
fi

if [ "$update" = "1" ] && [ "$api_failed" -ne 0 ]; then
  echo "❌ not writing expected.txt: the public API changed"
  exit 1
fi

if [ "$update" = "1" ]; then
  cp "$work/rust.txt" "$expected"
  echo "✓ wrote $expected from the Rust run"
  echo "  review the diff and commit it — this is what every language is measured against"
  exit 0
fi

failed=0

if [ ! -f "$expected" ]; then
  echo "expected.txt is missing - create it with: make check-api-update"
  exit 1
fi

echo
echo "comparing Rust against the committed answer..."
compare rust "$work/rust.txt" "$expected" "expected.txt" || failed=1

for name in ${languages[@]+"${languages[@]}"}; do
  echo "comparing $name..."
  compare "$name" "$work/$name.txt" "$expected" "expected.txt" || failed=1
  compare "$name" "$work/$name.txt" "$work/rust.txt" "live Rust" || failed=1
done

echo
echo "reading the same file over HTTP, lazily"
echo
if "$root/scripts/check-remote.sh"; then
  :
else
  failed=1
fi

echo
if [ ${#missing[@]} -gt 0 ]; then
  echo "⚠  NOT CHECKED: ${missing[*]} — no runner in check-api/"
  echo "   those languages are not covered by this run"
fi

if [ "$api_failed" -ne 0 ]; then
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  echo "❌ check-api failed"
  exit 1
fi

echo "✓ check-api passed for: rust ${languages[*]-}"
