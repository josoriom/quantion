import hashlib
import os
import platform
import re
import shutil
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Optional
from setuptools import setup
from setuptools.command.build_py import build_py


def detect_platform() -> str:
    system  = sys.platform
    machine = platform.machine().lower()

    os_name = {"darwin": "macos", "linux": "linux", "win32": "windows"}.get(system)
    if not os_name:
        raise RuntimeError(f"Unsupported OS: {system}")

    if machine in ("arm64", "aarch64"):
        arch = "arm64"
    elif machine in ("amd64", "x86_64"):
        arch = "x86_64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    return f"{os_name}-{arch}"


RELEASE_URL = os.environ.get(
    "QUANTION_RELEASE_URL",
    "https://github.com/phenological/quantion/releases/download",
)


def read_version() -> str:
    text = (Path(__file__).parent / "quantion" / "__init__.py").read_text(encoding="utf-8")
    found = re.search(r'^__version__ = "(.+)"$', text, re.MULTILINE)
    if not found:
        raise RuntimeError("quantion/__init__.py has no __version__")
    return found.group(1)


def library_extension(platform_dir: str) -> str:
    if platform_dir.startswith("windows"):
        return ".dll"
    if platform_dir.startswith("macos"):
        return ".dylib"
    return ".so"


def find_local_artifacts(platform_dir: str) -> Optional[Path]:
    here = Path(__file__).parent.resolve()
    artifacts_root = (here / ".." / ".." / "artifacts").resolve()
    direct = artifacts_root / platform_dir
    if direct.is_dir():
        return direct
    if not artifacts_root.is_dir():
        return None
    versions = sorted(
        (p for p in artifacts_root.iterdir() if (p / platform_dir).is_dir()),
        key=lambda p: tuple(int(part) if part.isdigit() else 0 for part in p.name.split(".")),
        reverse=True,
    )
    return versions[0] / platform_dir if versions else None


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def expected_checksum(version: str, asset: str) -> Optional[str]:
    url = f"{RELEASE_URL}/v{version}/SHA256SUMS.txt"
    try:
        with urllib.request.urlopen(url) as response:
            listing = response.read().decode("utf-8")
    except (urllib.error.URLError, OSError):
        return None
    for line in listing.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[1] == asset:
            return parts[0]
    return None


def download_native_binary(platform_dir: str, dst: Path) -> None:
    version = read_version()
    extension = library_extension(platform_dir)
    asset = f"libquantion-{platform_dir}{extension}"
    url = f"{RELEASE_URL}/v{version}/{asset}"
    dst.mkdir(parents=True, exist_ok=True)
    target = dst / f"libquantion{extension}"
    try:
        urllib.request.urlretrieve(url, target)
    except (urllib.error.URLError, OSError) as error:
        raise RuntimeError(f"quantion: failed to download {url}: {error}") from error
    expected = expected_checksum(version, asset)
    if expected is not None and expected != sha256_of(target):
        target.unlink()
        raise RuntimeError(f"quantion: checksum mismatch for {asset}")
    print(f"[quantion] downloaded {asset} from release v{version}")


def copy_native_binary(platform_dir: str, dst_root: Path) -> None:
    dst = dst_root / "quantion" / "native" / platform_dir
    src = find_local_artifacts(platform_dir)
    if src is None:
        download_native_binary(platform_dir, dst)
        return

    dst.mkdir(parents=True, exist_ok=True)
    copied = []
    for f in src.iterdir():
        if f.is_file():
            shutil.copy2(f, dst / f.name)
            copied.append(f.name)

    if not copied:
        raise FileNotFoundError(f"No files found in {src}")

    print(f"[quantion] copied {platform_dir}: {', '.join(copied)}")


class BuildWithNative(build_py):
    def run(self):
        platform_dir = detect_platform()
        build_root = Path(self.build_lib).resolve()
        copy_native_binary(platform_dir, build_root)
        super().run()


setup(cmdclass={"build_py": BuildWithNative})