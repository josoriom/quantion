#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output="$(mkdir -p "${1:-dist/r}" && cd "${1:-dist/r}" && pwd)"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

(cd "$root/wrappers" && tar -cf - --exclude=r/src/rust r) | tar -xf - -C "$work"
package="$work/r"
sh "$root/scripts/stage-r-rust.sh" "$package"
rust="$package/src/rust"

echo "vendoring the Rust crates..."
(cd "$rust" && cargo vendor --quiet vendor > /dev/null)
cat > "$rust/vendor-config.toml" <<'TOML'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
TOML

mkdir -p "$package/inst"
(cd "$rust" && cargo metadata --format-version 1) | python3 -c '
import json, sys
packages = json.load(sys.stdin)["packages"]
print("The R package quantion includes these Rust crates, vendored in src/rust/vendor.tar.xz.\n")
for package in sorted(packages, key=lambda item: item["name"]):
    if package["name"] == "quantion":
        continue
    name, version, license = package["name"], package["version"], package["license"]
    authors = ", ".join(package["authors"]) or "unknown"
    print(f"{name} {version} ({license}): {authors}")
' > "$package/inst/AUTHORS"

(cd "$rust" && tar -cJf vendor.tar.xz vendor && rm -rf vendor target)

echo "building the R package..."
(cd "$work" && R CMD build --no-build-vignettes r > build.log 2>&1) || { tail -20 "$work/build.log"; exit 1; }
tarball="$(ls "$work"/quantion_*.tar.gz)"

echo "checking $(basename "$tarball") as CRAN does..."
(cd "$work" && _R_CHECK_FORCE_SUGGESTS_=false R CMD check --as-cran --no-manual "$tarball" > check.log 2>&1) || true
grep -E "^Status:|WARNING|ERROR|NOTE" "$work/check.log" || true
if ! grep -qE "^Status: (OK|[0-9]+ NOTEs?)$" "$work/check.log"; then
  cp "$work/quantion.Rcheck/00check.log" "$output/check.log"
  awk '/^\* / { show = /(WARNING|ERROR)$/ } show' "$output/check.log"
  if [ -f "$work/quantion.Rcheck/00install.out" ]; then
    grep -iE "warning|error" "$work/quantion.Rcheck/00install.out" || true
  fi
  echo "the check failed, see $output/check.log"
  exit 1
fi

cp "$tarball" "$output/"
echo "built $output/$(basename "$tarball")"
