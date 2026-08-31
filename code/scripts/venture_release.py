#!/usr/bin/env python3
"""Validate Venture's coordinated semantic release version."""

from __future__ import annotations

import argparse
import re
import tomllib
from pathlib import Path


SEMVER = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)
VERSION_FILE = Path("code/programs/mosaic/venture-browser/VERSION")
MANIFESTS = (
    Path("code/packages/rust/venture-browser-core/Cargo.toml"),
    Path("code/packages/rust/venture-browser-cairo/Cargo.toml"),
    Path("code/packages/rust/venture-browser-qt/Cargo.toml"),
    Path("code/packages/rust/venture-browser-macos/Cargo.toml"),
    Path("code/packages/rust/venture-browser-windows/Cargo.toml"),
    Path("code/programs/mosaic/venture-browser/Cargo.toml"),
    Path("code/programs/mosaic/venture-browser/mosaic-package.toml"),
)


def validate_release(repo_root: Path, tag: str | None = None) -> str:
    version = (repo_root / VERSION_FILE).read_text(encoding="utf-8").strip()
    match = SEMVER.fullmatch(version)
    if match is None:
        raise ValueError(f"Venture VERSION is not SemVer: {version!r}")
    if match.group(1) != "0":
        raise ValueError("Venture must remain on a pre-1.0 version until readiness is declared")

    mismatches = []
    for relative_path in MANIFESTS:
        manifest = tomllib.loads((repo_root / relative_path).read_text(encoding="utf-8"))
        manifest_version = manifest["package"]["version"]
        if manifest_version != version:
            mismatches.append(f"{relative_path}: {manifest_version}")
    if mismatches:
        details = ", ".join(mismatches)
        raise ValueError(f"Venture manifests do not match {version}: {details}")

    expected_tag = f"venture-v{version}"
    if tag is not None and tag != expected_tag:
        raise ValueError(f"release tag must be {expected_tag}, got {tag}")
    return version


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--tag")
    args = parser.parse_args()
    version = validate_release(args.repo_root.resolve(), args.tag)
    print(version)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
