#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="${1:-}"
[ -n "$version" ] || { echo "usage: make publish_python VERSION=X.Y.Z   (after make publish VERSION=X.Y.Z)"; exit 1; }
version="${version#v}"
tag="v$version"

git fetch --quiet --tags origin
git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null || {
  echo "tag $tag does not exist, run make publish VERSION=$version first"; exit 1
}

tagged_version="$(git show "$tag:wrappers/python/pyproject.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)"
[ "$tagged_version" = "$version" ] || { echo "$tag has python version '$tagged_version', not $version"; exit 1; }

repository="$(git remote get-url origin | sed -E 's#^.*github\.com[:/]##; s#\.git$##')"

libraries="$(gh release view "$tag" --repo "$repository" --json assets --jq '[.assets[].name | select(startswith("libquantion-"))] | length' 2>/dev/null || echo 0)"
[ "$libraries" -ge 5 ] || { echo "release $tag has $libraries of 5 binaries, wait for the release workflow to finish"; exit 1; }

if curl --silent --fail --output /dev/null "https://pypi.org/pypi/quantion/$version/json"; then
  echo "quantion $version is already on PyPI"; exit 1
fi

gh workflow run publish-python.yml --repo "$repository" --ref main -f tag="$tag"
echo "started the Publish Python workflow for $tag: https://github.com/$repository/actions/workflows/publish-python.yml"
