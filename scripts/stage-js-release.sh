#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="${1:-}"
[ -n "$version" ] || { echo "usage: scripts/stage-js-release.sh X.Y.Z"; exit 1; }
version="${version#v}"
tag="v$version"

repository="$(git remote get-url origin | sed -E 's#^.*github\.com[:/]##; s#\.git$##')"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "downloading the binaries of release $tag..."
gh release download "$tag" --repo "$repository" --dir "$work" \
  --pattern 'libquantion-*' --pattern 'quantion-*.node' --pattern 'quantion.wasm' --pattern 'SHA256SUMS.txt' || {
  echo "release $tag has no binaries yet, wait for the release workflow to finish"; exit 1
}

echo "checking the checksums..."
(cd "$work" && grep -E ' (libquantion-|quantion-.*\.node$|quantion\.wasm$)' SHA256SUMS.txt | shasum -a 256 --check --quiet)

artifacts="artifacts/$version"
for asset in "$work"/libquantion-*; do
  name="$(basename "$asset")"
  platform="${name#libquantion-}"
  platform="${platform%.*}"
  [ -f "$work/quantion-$platform.node" ] || { echo "release $tag has no quantion-$platform.node"; exit 1; }
  mkdir -p "$artifacts/$platform"
  cp "$asset" "$artifacts/$platform/libquantion.${name##*.}"
  cp "$work/quantion-$platform.node" "$artifacts/$platform/quantion.node"
done
mkdir -p "$artifacts/wasm"
cp "$work/quantion.wasm" "$artifacts/wasm/quantion.wasm"
echo "staged the binaries of $tag in $artifacts"
