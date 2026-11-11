#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="${1:-}"
[ -n "$version" ] || { echo "usage: scripts/build-python-wheels.sh X.Y.Z [output folder]"; exit 1; }
version="${version#v}"
tag="v$version"

git fetch --quiet --tags origin
git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null || {
  echo "tag $tag does not exist, run make publish VERSION=$version first"; exit 1
}

tagged_version="$(git show "$tag:wrappers/python/pyproject.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)"
[ "$tagged_version" = "$version" ] || { echo "$tag has python version '$tagged_version', not $version"; exit 1; }

repository="$(git remote get-url origin | sed -E 's#^.*github\.com[:/]##; s#\.git$##')"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
output="${2:-dist/python/$version}"

echo "downloading the binaries of release $tag..."
gh release download "$tag" --repo "$repository" --dir "$work/release" --pattern 'libquantion-*' --pattern 'SHA256SUMS.txt' || {
  echo "release $tag has no binaries yet, wait for the release workflow to finish"; exit 1
}

mkdir -p "$work/source"
git archive "$tag" wrappers/python | tar -x -C "$work/source"
for asset in "$work"/release/libquantion-*; do
  name="$(basename "$asset")"
  platform="${name#libquantion-}"
  platform="${platform%.*}"
  mkdir -p "$work/source/artifacts/$platform"
  cp "$asset" "$work/source/artifacts/$platform/libquantion.${name##*.}"
done

echo "building the wheel of $tag..."
python3 -m pip wheel "$work/source/wrappers/python" --no-deps --wheel-dir "$work/wheel" --quiet

rm -rf "$output"
python3 - "$work/wheel" "$work/release" "$output" <<'PY'
import base64
import hashlib
import re
import sys
import zipfile
from pathlib import Path

wheel_dir, release_dir, output_dir = (Path(path) for path in sys.argv[1:4])
output_dir.mkdir(parents=True, exist_ok=True)

platforms = {
    "macos-arm64": ("libquantion.dylib", "macosx_11_0_arm64"),
    "macos-x86_64": ("libquantion.dylib", "macosx_10_12_x86_64"),
    "linux-x86_64": ("libquantion.so", "manylinux_2_{glibc}_x86_64"),
    "linux-arm64": ("libquantion.so", "manylinux_2_{glibc}_aarch64"),
    "windows-x86_64": ("libquantion.dll", "win_amd64"),
}


def get_digest(data):
    return "sha256=" + base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()


def get_glibc(library):
    return max(int(minor) for minor in re.findall(rb"GLIBC_2\.(\d+)", library))


checksums = {}
for line in (release_dir / "SHA256SUMS.txt").read_text().splitlines():
    if line.strip():
        digest, name = line.split()
        checksums[name] = digest

base_wheel = next(wheel_dir.glob("quantion-*.whl"))
with zipfile.ZipFile(base_wheel) as source:
    files = {name: source.read(name) for name in source.namelist() if not name.startswith("quantion/native/")}
record_name = next(name for name in files if name.endswith(".dist-info/RECORD"))
wheel_name = next(name for name in files if name.endswith(".dist-info/WHEEL"))
name_version = base_wheel.name.split("-py3-")[0]

for platform_dir, (library_name, tag_pattern) in platforms.items():
    asset = release_dir / f"libquantion-{platform_dir}{Path(library_name).suffix}"
    library = asset.read_bytes()
    if hashlib.sha256(library).hexdigest() != checksums.get(asset.name):
        sys.exit(f"checksum mismatch for {asset.name}")
    platform_tag = tag_pattern.format(glibc=get_glibc(library)) if "{glibc}" in tag_pattern else tag_pattern

    wheel_files = {name: data for name, data in files.items() if name != record_name}
    wheel_files[f"quantion/native/{platform_dir}/{library_name}"] = library
    wheel_files[wheel_name] = re.sub(rb"Tag: .*", f"Tag: py3-none-{platform_tag}".encode(), files[wheel_name])
    record = "".join(f"{name},{get_digest(data)},{len(data)}\n" for name, data in wheel_files.items())
    wheel_files[record_name] = (record + f"{record_name},,\n").encode()

    target = output_dir / f"{name_version}-py3-none-{platform_tag}.whl"
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as wheel:
        for name, data in wheel_files.items():
            wheel.writestr(name, data)
    print(f"built {target}")
PY
