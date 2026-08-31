#!/usr/bin/env python3
"""Validate and assemble incremental TaskApp GitHub release metadata."""

from __future__ import annotations

import argparse
import io
import json
import plistlib
import re
import struct
import sys
import tarfile
import zipfile
import zlib
from datetime import datetime
from pathlib import Path
from typing import Any

TAG_PREFIX = "task-app-v"
SEMVER = re.compile(
    r"^(0|[1-9]\d*)\."
    r"(0|[1-9]\d*)\."
    r"(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)
COMMIT = re.compile(r"^[0-9a-fA-F]{40}$")
REPOSITORY = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")

NATIVE_TARGETS: dict[str, dict[str, str]] = {
    "qt": {
        "artifact_label": "qt-linux",
        "platform": "Linux x86_64",
        "toolkit": "Qt",
    },
    "flutter": {
        "artifact_label": "flutter-linux",
        "platform": "Linux x86_64",
        "toolkit": "Flutter",
    },
    "compose": {
        "artifact_label": "compose-linux",
        "platform": "Linux x86_64",
        "toolkit": "Compose Desktop",
    },
    "swiftui": {
        "artifact_label": "swiftui-macos",
        "platform": "macOS",
        "toolkit": "SwiftUI",
    },
    "xaml": {
        "artifact_label": "xaml-windows",
        "platform": "Windows",
        "toolkit": "WinUI/XAML",
    },
}

LINUX_BUNDLES: dict[str, dict[str, str]] = {
    "qt": {
        "artifact_label": "qt-linux",
        "toolkit": "Qt",
        "state_path": "TaskApp/task-app/mosaic-state.v1.json",
        "prerequisites": "A compatible Linux x86_64 system with Qt 6.8 libraries.",
    },
    "flutter": {
        "artifact_label": "flutter-linux",
        "toolkit": "Flutter",
        "state_path": "task-app/mosaic-state.v1.json",
        "prerequisites": "A compatible Linux x86_64 system with GTK 3 libraries.",
    },
    "compose": {
        "artifact_label": "compose-linux",
        "toolkit": "Compose Desktop",
        "state_path": "task-app/mosaic-state.v1.json",
        "prerequisites": "A compatible glibc-based Linux x86_64 system.",
    },
}


def validate_identifiers(version: str, tag: str, commit: str | None = None) -> None:
    """Reject invalid or mismatched release identifiers."""

    if SEMVER.fullmatch(version) is None:
        raise ValueError(f"version is not strict SemVer: {version!r}")
    expected_tag = f"{TAG_PREFIX}{version}"
    if tag != expected_tag:
        raise ValueError(f"tag must be {expected_tag!r}, got {tag!r}")
    if commit is not None and COMMIT.fullmatch(commit) is None:
        raise ValueError("commit must be a full 40-character Git SHA")


def artifact_names(version: str) -> list[str]:
    """Return the complete, stable payload set for one release."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    names = [f"task-app-web-v{version}.zip"]
    names.extend(
        f"task-app-{target['artifact_label']}-project-v{version}.zip"
        for target in NATIVE_TARGETS.values()
    )
    names.extend(
        f"task-app-{target['artifact_label']}-bundle-v{version}.tar.gz"
        for target in LINUX_BUNDLES.values()
    )
    names.append(f"task-app-swiftui-macos-bundle-v{version}.zip")
    names.append(f"task-app-xaml-windows-bundle-v{version}.zip")
    return names


def _zip_tree(source: Path, output: Path, root_name: str, commit: str) -> None:
    if not source.is_dir():
        raise ValueError(f"archive source directory does not exist: {source}")
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr(f"{root_name}/SOURCE_COMMIT", f"{commit}\n")
        for path in sorted(source.rglob("*")):
            if path.is_file():
                relative = path.relative_to(source).as_posix()
                archive.write(path, f"{root_name}/{relative}")


def archive_web(version: str, commit: str, source: Path, output_dir: Path) -> Path:
    """Verify and archive the production web bundle."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    required = (source / "index.html", source / "task_engine.wasm")
    missing = [str(path) for path in required if not path.is_file()]
    if not (source / "assets").is_dir():
        missing.append(str(source / "assets"))
    if missing:
        raise ValueError(f"web bundle is incomplete: {', '.join(missing)}")
    output = output_dir / f"task-app-web-v{version}.zip"
    _zip_tree(source, output, f"task-app-web-v{version}", commit)
    return output


def archive_native(
    version: str,
    commit: str,
    backend: str,
    source: Path,
    output_dir: Path,
) -> Path:
    """Verify and archive one strict generated native project."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if backend not in NATIVE_TARGETS:
        raise ValueError(f"unsupported native release backend: {backend}")
    report_path = source / "mosaic-degradations.json"
    if not report_path.is_file():
        raise ValueError(f"missing native-complete report: {report_path}")
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if report.get("nativeComplete") is not True or report.get("degradations") != []:
        raise ValueError(f"{backend} project is not strict native-complete")
    label = NATIVE_TARGETS[backend]["artifact_label"]
    output = output_dir / f"task-app-{label}-project-v{version}.zip"
    _zip_tree(source, output, f"task-app-{label}-project-v{version}", commit)
    return output


def _relative_bundle_path(source: Path, candidate: Path, label: str) -> Path:
    source_root = source.resolve(strict=True)
    resolved = candidate.resolve(strict=True)
    try:
        return resolved.relative_to(source_root)
    except ValueError as error:
        raise ValueError(f"{label} must be inside the bundle source: {candidate}") from error


def _add_text_member(
    archive: tarfile.TarFile,
    name: str,
    content: str,
    mode: int = 0o644,
) -> None:
    payload = content.encode("utf-8")
    info = tarfile.TarInfo(name)
    info.size = len(payload)
    info.mode = mode
    info.mtime = 0
    archive.addfile(info, io.BytesIO(payload))


def _linux_launcher(version: str, backend: str, executable: str) -> str:
    state_path = LINUX_BUNDLES[backend]["state_path"]
    return f"""#!/bin/sh
set -eu

BUNDLE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
STATE_ROOT=${{XDG_DATA_HOME:-${{HOME:?HOME is required}}/.local/share}}
STATE_FILE="$STATE_ROOT/{state_path}"
BACKUP_DIR="$STATE_ROOT/task-app/backups"
BACKUP_FILE="$BACKUP_DIR/pre-v{version}-{backend}.json"

if [ -f "$STATE_FILE" ] && [ ! -e "$BACKUP_FILE" ]; then
  mkdir -p "$BACKUP_DIR"
  TEMP_BACKUP="$BACKUP_FILE.tmp.$$"
  cp -- "$STATE_FILE" "$TEMP_BACKUP"
  mv -- "$TEMP_BACKUP" "$BACKUP_FILE"
fi

exec "$BUNDLE_DIR/{executable}" "$@"
"""


def archive_linux_bundle(
    version: str,
    commit: str,
    backend: str,
    source: Path,
    executable: Path,
    runtime: Path,
    expected_runtime: Path,
    output_dir: Path,
) -> Path:
    """Verify and archive one runnable Linux application tree."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if backend not in LINUX_BUNDLES:
        raise ValueError(f"unsupported Linux bundle backend: {backend}")
    if not source.is_dir():
        raise ValueError(f"bundle source directory does not exist: {source}")
    executable_relative = _relative_bundle_path(source, executable, "executable")
    runtime_relative = _relative_bundle_path(source, runtime, "runtime")
    if not executable.is_file():
        raise ValueError(f"bundle executable does not exist: {executable}")
    if not runtime.is_file() or not expected_runtime.is_file():
        raise ValueError("bundled and expected Rust runtime files must exist")
    if runtime.read_bytes() != expected_runtime.read_bytes():
        raise ValueError("bundled Rust runtime does not match the selected build artifact")

    source_root = source.resolve(strict=True)
    for path in source.rglob("*"):
        try:
            path.resolve(strict=True).relative_to(source_root)
        except ValueError as error:
            raise ValueError(f"bundle contains a path outside its source: {path}") from error

    target = LINUX_BUNDLES[backend]
    root_name = f"task-app-{target['artifact_label']}-bundle-v{version}"
    output = output_dir / f"{root_name}.tar.gz"
    output.parent.mkdir(parents=True, exist_ok=True)
    executable_name = executable_relative.as_posix()
    runtime_name = runtime_relative.as_posix()
    metadata = {
        "schemaVersion": 1,
        "product": "Trestle",
        "applicationId": "task-app",
        "version": version,
        "sourceCommit": commit.lower(),
        "platform": "Linux x86_64",
        "backend": backend,
        "toolkit": target["toolkit"],
        "executable": executable_name,
        "rustRuntime": runtime_name,
        "statePath": f"$XDG_DATA_HOME/{target['state_path']}",
        "launcher": "launch-trestle",
    }
    instructions = f"""Trestle {version} — {target['toolkit']} portable Linux bundle

This is an unpack-and-run bundle for compatible Linux x86_64 systems. It is not
a distribution-native installer and it is not signed.

Prerequisite: {target['prerequisites']}

Run from any working directory:
  /path/to/{root_name}/launch-trestle

The launcher preserves one version-and-backend-specific pre-upgrade copy of an
existing local state file under $XDG_DATA_HOME/task-app/backups (or
~/.local/share/task-app/backups) before starting Trestle. The app continues to
own its live state under the stable application identity `task-app`.
"""
    launcher = _linux_launcher(version, backend, executable_name)
    with tarfile.open(output, "w:gz", format=tarfile.PAX_FORMAT) as archive:
        archive.add(source, arcname=root_name, recursive=True)
        _add_text_member(archive, f"{root_name}/SOURCE_COMMIT", f"{commit.lower()}\n")
        _add_text_member(
            archive,
            f"{root_name}/BUNDLE.json",
            json.dumps(metadata, indent=2, sort_keys=True) + "\n",
        )
        _add_text_member(archive, f"{root_name}/INSTALL.txt", instructions)
        _add_text_member(
            archive,
            f"{root_name}/launch-trestle",
            launcher,
            mode=0o755,
        )
    return output


def _png_chunk(chunk_type: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + chunk_type
        + data
        + struct.pack(">I", zlib.crc32(chunk_type + data) & 0xFFFFFFFF)
    )


def _trestle_icon_png(size: int) -> bytes:
    """Render a dependency-free square Trestle bridge mark for an ICNS chunk."""

    rows = bytearray()
    for y in range(size):
        rows.append(0)
        for x in range(size):
            margin = size * 0.17
            post = size * 0.075
            deck_y = size * 0.67
            arch_center = size * 0.5
            arch_radius = size * 0.28
            arch_y = size * 0.34 + ((x - arch_center) ** 2) / (arch_radius * 2.4)
            is_post = (
                (margin <= x <= margin + post or size - margin - post <= x <= size - margin)
                and size * 0.36 <= y <= deck_y
            )
            is_deck = deck_y - size * 0.035 <= y <= deck_y + size * 0.035
            is_arch = margin <= x <= size - margin and abs(y - arch_y) <= size * 0.035
            if is_post or is_deck or is_arch:
                color = (245, 247, 250, 255)
            elif y >= size * 0.78:
                color = (27, 105, 138, 255)
            else:
                color = (22, 41, 70, 255)
            rows.extend(color)
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", header)
        + _png_chunk(b"IDAT", zlib.compress(bytes(rows), level=9))
        + _png_chunk(b"IEND", b"")
    )


def _trestle_icns() -> bytes:
    chunks = []
    for kind, size in (
        (b"icp4", 16),
        (b"icp5", 32),
        (b"icp6", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic09", 512),
        (b"ic10", 1024),
    ):
        payload = _trestle_icon_png(size)
        chunks.append(kind + struct.pack(">I", len(payload) + 8) + payload)
    body = b"".join(chunks)
    return b"icns" + struct.pack(">I", len(body) + 8) + body


def _trestle_ico() -> bytes:
    images = [_trestle_icon_png(size) for size in (16, 32, 48, 64, 128, 256)]
    header_size = 6 + 16 * len(images)
    entries = []
    offset = header_size
    for size, payload in zip((16, 32, 48, 64, 128, 256), images, strict=True):
        encoded_size = 0 if size == 256 else size
        entries.append(
            struct.pack(
                "<BBBBHHII",
                encoded_size,
                encoded_size,
                0,
                0,
                1,
                32,
                len(payload),
                offset,
            )
        )
        offset += len(payload)
    return struct.pack("<HHH", 0, 1, len(images)) + b"".join(entries) + b"".join(images)


def write_windows_icon(output: Path) -> Path:
    """Write the stable multi-resolution Trestle Windows application icon."""

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(_trestle_ico())
    return output


def _add_zip_member(
    archive: zipfile.ZipFile,
    name: str,
    payload: bytes,
    mode: int = 0o644,
) -> None:
    info = zipfile.ZipInfo(name)
    info.create_system = 3
    info.external_attr = mode << 16
    archive.writestr(info, payload, compress_type=zipfile.ZIP_DEFLATED)


def archive_macos_app(
    version: str,
    commit: str,
    architecture: str,
    executable: Path,
    resource_bundle: Path,
    runtime: Path,
    expected_runtime: Path,
    output_dir: Path,
) -> Path:
    """Assemble and archive the unsigned SwiftUI Trestle.app bundle."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if architecture not in {"arm64", "x86_64"}:
        raise ValueError(f"unsupported macOS architecture: {architecture}")
    if not executable.is_file():
        raise ValueError(f"macOS executable does not exist: {executable}")
    if not resource_bundle.is_dir():
        raise ValueError(f"SwiftPM resource bundle does not exist: {resource_bundle}")
    if resource_bundle.name != "App_App.bundle":
        raise ValueError("SwiftPM resource bundle must be named App_App.bundle")
    runtime_relative = _relative_bundle_path(resource_bundle, runtime, "runtime")
    if runtime_relative.as_posix() != "Runtime/libmosaic_app.dylib":
        raise ValueError("macOS Rust runtime must be Runtime/libmosaic_app.dylib")
    if not runtime.is_file() or not expected_runtime.is_file():
        raise ValueError("bundled and expected macOS Rust runtime files must exist")
    if runtime.read_bytes() != expected_runtime.read_bytes():
        raise ValueError("bundled macOS Rust runtime does not match the selected build artifact")

    bundle_root = resource_bundle.resolve(strict=True)
    for path in resource_bundle.rglob("*"):
        try:
            path.resolve(strict=True).relative_to(bundle_root)
        except ValueError as error:
            raise ValueError(
                f"SwiftPM resource bundle contains an external path: {path}"
            ) from error

    core_version = version.split("-", 1)[0].split("+", 1)[0]
    info_plist = {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleDisplayName": "Trestle",
        "CFBundleExecutable": "Trestle",
        "CFBundleIconFile": "Trestle",
        "CFBundleIdentifier": "org.codingadventures.trestle",
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": "Trestle",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": core_version,
        "LSApplicationCategoryType": "public.app-category.productivity",
        "LSMinimumSystemVersion": "13.0",
        "MosaicApplicationID": "task-app",
        "NSHighResolutionCapable": True,
    }
    metadata = {
        "schemaVersion": 1,
        "product": "Trestle",
        "applicationId": "task-app",
        "bundleIdentifier": "org.codingadventures.trestle",
        "version": version,
        "sourceCommit": commit.lower(),
        "platform": "macOS 13+",
        "architecture": architecture,
        "toolkit": "SwiftUI",
        "executable": "Contents/MacOS/Trestle",
        "rustRuntime": f"{resource_bundle.name}/Runtime/libmosaic_app.dylib",
        "signed": False,
        "notarized": False,
        "iosArtifact": False,
    }
    instructions = f"""Trestle {version} — unsigned macOS application bundle

Unzip the archive, then open Trestle.app on macOS 13 or newer. This development
bundle is neither signed nor notarized. macOS may require an explicit first-open
approval in Privacy & Security. Do not weaken system-wide Gatekeeper settings.

The bundled dylib is the macOS Rust engine selected by the release workflow. The
SwiftUI source project remains portable to iOS, but this .app and its dylib are
macOS artifacts and are not presented as an iOS build.
"""
    output = output_dir / f"task-app-swiftui-macos-bundle-v{version}.zip"
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w") as archive:
        _add_zip_member(
            archive,
            "Trestle.app/Contents/MacOS/Trestle",
            executable.read_bytes(),
            mode=0o755,
        )
        for path in sorted(resource_bundle.rglob("*")):
            if path.is_file():
                relative = path.relative_to(resource_bundle).as_posix()
                archive.write(path, f"Trestle.app/{resource_bundle.name}/{relative}")
        _add_zip_member(
            archive,
            "Trestle.app/Contents/Info.plist",
            plistlib.dumps(info_plist, fmt=plistlib.FMT_XML, sort_keys=True),
        )
        _add_zip_member(
            archive,
            "Trestle.app/Contents/Resources/Trestle.icns",
            _trestle_icns(),
        )
        _add_zip_member(
            archive,
            "Trestle.app/Contents/Resources/SOURCE_COMMIT",
            f"{commit.lower()}\n".encode(),
        )
        _add_zip_member(
            archive,
            "Trestle.app/Contents/Resources/BUNDLE.json",
            (json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode(),
        )
        _add_zip_member(
            archive,
            "Trestle.app/Contents/Resources/INSTALL.txt",
            instructions.encode(),
        )
    return output


def archive_windows_app(
    version: str,
    commit: str,
    source: Path,
    executable: Path,
    runtime: Path,
    expected_runtime: Path,
    output_dir: Path,
) -> Path:
    """Verify and archive the self-contained unpackaged Windows application."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if not source.is_dir():
        raise ValueError(f"Windows publish directory does not exist: {source}")
    executable_relative = _relative_bundle_path(source, executable, "executable")
    runtime_relative = _relative_bundle_path(source, runtime, "runtime")
    if executable_relative.as_posix() != "Trestle.exe":
        raise ValueError("Windows release executable must be Trestle.exe")
    if runtime_relative.as_posix() != "mosaic_app.dll":
        raise ValueError("Windows Rust runtime must be mosaic_app.dll beside Trestle.exe")
    if not executable.is_file() or not runtime.is_file() or not expected_runtime.is_file():
        raise ValueError("Windows executable and Rust runtime files must exist")
    if runtime.read_bytes() != expected_runtime.read_bytes():
        raise ValueError("bundled Windows Rust runtime does not match the selected build artifact")

    source_root = source.resolve(strict=True)
    for path in source.rglob("*"):
        try:
            path.resolve(strict=True).relative_to(source_root)
        except ValueError as error:
            raise ValueError(f"Windows bundle contains an external path: {path}") from error

    root_name = f"Trestle-windows-x64-v{version}"
    metadata = {
        "schemaVersion": 1,
        "product": "Trestle",
        "applicationId": "task-app",
        "applicationIdentity": "org.codingadventures.trestle",
        "version": version,
        "sourceCommit": commit.lower(),
        "platform": "Windows 10 2004+",
        "architecture": "x64",
        "toolkit": "WinUI 3 / XAML",
        "executable": "Trestle.exe",
        "rustRuntime": "mosaic_app.dll",
        "statePath": "%LOCALAPPDATA%\\task-app\\mosaic-state.v1.json",
        "dotnetSelfContained": True,
        "windowsAppSdkSelfContained": True,
        "signed": False,
        "msix": False,
    }
    instructions = f"""Trestle {version} — portable Windows x64 application

Extract the entire directory and run Trestle.exe on Windows 10 version 2004 or
newer. Keep all files together. This release carries its .NET and Windows App SDK
runtimes, so it does not require a separate framework install.

This is an unsigned, unpackaged development build rather than an MSIX installer.
Windows SmartScreen may require an explicit first-run confirmation. Do not disable
system-wide security controls. Local task state remains under the stable
%LOCALAPPDATA%\\task-app identity.
"""
    output = output_dir / f"task-app-xaml-windows-bundle-v{version}.zip"
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(source.rglob("*")):
            if path.is_file():
                relative = path.relative_to(source).as_posix()
                archive.write(path, f"{root_name}/{relative}")
        _add_zip_member(
            archive,
            f"{root_name}/Trestle.ico",
            _trestle_ico(),
        )
        _add_zip_member(
            archive,
            f"{root_name}/SOURCE_COMMIT",
            f"{commit.lower()}\n".encode(),
        )
        _add_zip_member(
            archive,
            f"{root_name}/BUNDLE.json",
            (json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode(),
        )
        _add_zip_member(
            archive,
            f"{root_name}/INSTALL.txt",
            instructions.encode(),
        )
    return output


def build_manifest(
    version: str,
    tag: str,
    commit: str,
    assets_dir: Path,
) -> dict[str, Any]:
    """Build provenance and platform coverage for the exact payload set."""

    validate_identifiers(version, tag, commit)
    expected = artifact_names(version)
    actual = sorted(path.name for path in assets_dir.iterdir() if path.is_file())
    if actual != sorted(expected):
        raise ValueError(
            f"release payload mismatch: expected {sorted(expected)}, got {actual}"
        )

    artifacts: list[dict[str, Any]] = [
        {
            "name": f"task-app-web-v{version}.zip",
            "kind": "production-web-bundle",
            "platform": "Modern browsers",
            "installable": False,
            "verification": "Vitest, TypeScript, Vite production build, and WASM presence",
        }
    ]
    for target in NATIVE_TARGETS.values():
        artifacts.append(
            {
                "name": (f"task-app-{target['artifact_label']}-project-v{version}.zip"),
                "kind": "generated-native-project",
                "platform": target["platform"],
                "toolkit": target["toolkit"],
                "installable": False,
                "verification": (
                    "strict native-complete generation, bundled Rust runtime, "
                    "and emitted-control contract"
                ),
            }
        )
    for backend, target in LINUX_BUNDLES.items():
        artifacts.append(
            {
                "name": f"task-app-{target['artifact_label']}-bundle-v{version}.tar.gz",
                "kind": "portable-linux-bundle",
                "platform": "Linux x86_64",
                "toolkit": target["toolkit"],
                "installable": False,
                "runnable": True,
                "applicationId": "task-app",
                "verification": (
                    "release build, byte-identical bundled Rust runtime, "
                    "unrelated-working-directory launch, and pre-upgrade snapshot contract"
                ),
                "backend": backend,
            }
        )
    artifacts.append(
        {
            "name": f"task-app-swiftui-macos-bundle-v{version}.zip",
            "kind": "unsigned-macos-application",
            "platform": "macOS 13+ / release-runner architecture",
            "toolkit": "SwiftUI",
            "installable": False,
            "runnable": True,
            "signed": False,
            "notarized": False,
            "applicationId": "task-app",
            "bundleIdentifier": "org.codingadventures.trestle",
            "verification": (
                "release build, byte-identical bundled Rust runtime, unrelated-"
                "working-directory launch, and upgrade-style state restoration"
            ),
        }
    )
    artifacts.append(
        {
            "name": f"task-app-xaml-windows-bundle-v{version}.zip",
            "kind": "portable-windows-application",
            "platform": "Windows 10 2004+ / x64",
            "toolkit": "WinUI 3 / XAML",
            "installable": False,
            "runnable": True,
            "signed": False,
            "msix": False,
            "applicationId": "task-app",
            "applicationIdentity": "org.codingadventures.trestle",
            "verification": (
                "self-contained publish, byte-identical bundled Rust runtime, "
                "UI Automation launch, and replacement-state restoration"
            ),
        }
    )
    return {
        "schemaVersion": 1,
        "product": "TaskApp/Trestle",
        "version": version,
        "tag": tag,
        "sourceCommit": commit.lower(),
        "artifacts": artifacts,
        "knownLimitations": [
            "Linux bundles are portable archives, not signed distribution packages.",
            "The macOS app is unsigned and not notarized.",
            "The Windows app is unsigned and unpackaged; no MSIX is provided.",
            "Linux bundles require the compatible system libraries named in INSTALL.txt.",
        ],
    }


def _parse_timestamp(value: str | None) -> datetime | None:
    if not value:
        return None
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def _history_lines(history: list[dict[str, Any]], since: str | None) -> list[str]:
    since_time = _parse_timestamp(since)
    entries: list[tuple[datetime, str]] = []
    for pull_request in history:
        merged_at = _parse_timestamp(pull_request.get("mergedAt"))
        if merged_at is None or (since_time is not None and merged_at <= since_time):
            continue
        number = pull_request.get("number")
        title = pull_request.get("title")
        url = pull_request.get("url")
        if (
            not isinstance(number, int)
            or not isinstance(title, str)
            or not isinstance(url, str)
        ):
            continue
        entries.append((merged_at, f"- [{title} (#{number})]({url})"))
    entries.sort(key=lambda entry: entry[0], reverse=True)
    return [line for _, line in entries]


def render_notes(
    version: str,
    tag: str,
    commit: str,
    repository: str,
    history: list[dict[str, Any]],
    since: str | None,
) -> str:
    """Render honest, product-scoped notes from GitHub pull-request history."""

    validate_identifiers(version, tag, commit)
    if REPOSITORY.fullmatch(repository) is None:
        raise ValueError(f"invalid GitHub repository: {repository!r}")
    history_lines = _history_lines(history, since)
    history_section = (
        "\n".join(history_lines)
        if history_lines
        else "- No labeled TaskApp PRs in this interval."
    )
    issue_root = f"https://github.com/{repository}/issues"
    return f"""# TaskApp v{version}

This is an intentionally incremental TaskApp/Trestle release from commit
`{commit.lower()}`. Task data remains local, and scheduling is owned by the shared
Rust engine.

## What is usable now

- Add tasks with optional due dates, inspect the Rust-generated schedule, complete,
  reopen, and delete them.
- Restore the local workspace after a web reload or generated native app restart.
- Serve the production web ZIP from any static web server.
- Unpack and run a verified Linux bundle, or build a strict generated native
  project for Windows.
- Unzip and run the unsigned SwiftUI `Trestle.app` on the matching macOS 13+
  architecture recorded in its `BUNDLE.json`.
- Extract and run the self-contained Windows x64 `Trestle.exe` bundle.

## Artifact and platform coverage

| Artifact | Platform | Coverage |
| --- | --- | --- |
| `task-app-web-v{version}.zip` | Modern browsers | Tested production bundle; no installer required |
| `task-app-qt-linux-project-v{version}.zip` | Linux x86_64 / Qt | Generated native-complete project; no installer |
| `task-app-flutter-linux-project-v{version}.zip` | Linux x86_64 / Flutter | Generated native-complete project; no installer |
| `task-app-compose-linux-project-v{version}.zip` | Linux x86_64 / Compose Desktop | Generated native-complete project; no installer |
| `task-app-swiftui-macos-project-v{version}.zip` | macOS / SwiftUI | Generated native-complete project; no installer |
| `task-app-xaml-windows-project-v{version}.zip` | Windows / WinUI | Generated native-complete project; no installer |
| `task-app-qt-linux-bundle-v{version}.tar.gz` | Linux x86_64 / Qt | Verified portable bundle; compatible Qt 6.8 system required |
| `task-app-flutter-linux-bundle-v{version}.tar.gz` | Linux x86_64 / Flutter | Verified portable bundle; compatible GTK 3 system required |
| `task-app-compose-linux-bundle-v{version}.tar.gz` | Linux x86_64 / Compose Desktop | Verified portable bundle with bundled JVM runtime |
| `task-app-swiftui-macos-bundle-v{version}.zip` | macOS 13+ / runner-native architecture | Verified unsigned `Trestle.app`; not notarized |
| `task-app-xaml-windows-bundle-v{version}.zip` | Windows 10 2004+ / x64 | Verified self-contained portable app; unsigned, no MSIX |

`task-app-release-manifest-v{version}.json` records the source commit and exact
verification claim for every payload. `SHA256SUMS` authenticates every payload and
the manifest.

## Known limitations

- Linux payloads are portable archives rather than signed distribution packages.
- The macOS app is unsigned/not notarized; the Windows app is unsigned and unpackaged.
- Signing and installer lifecycle remain tracked in [#13522]({issue_root}/13522).
- Mobile binaries are not release artifacts in this version.

## TaskApp GitHub history

{history_section}
"""


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser("validate")
    validate_parser.add_argument("--version", required=True)
    validate_parser.add_argument("--tag", required=True)
    validate_parser.add_argument("--commit", required=True)

    web_parser = subparsers.add_parser("archive-web")
    web_parser.add_argument("--version", required=True)
    web_parser.add_argument("--commit", required=True)
    web_parser.add_argument("--source", type=Path, required=True)
    web_parser.add_argument("--output-dir", type=Path, required=True)

    native_parser = subparsers.add_parser("archive-native")
    native_parser.add_argument("--version", required=True)
    native_parser.add_argument("--commit", required=True)
    native_parser.add_argument("--backend", required=True)
    native_parser.add_argument("--source", type=Path, required=True)
    native_parser.add_argument("--output-dir", type=Path, required=True)

    bundle_parser = subparsers.add_parser("archive-linux-bundle")
    bundle_parser.add_argument("--version", required=True)
    bundle_parser.add_argument("--commit", required=True)
    bundle_parser.add_argument("--backend", required=True)
    bundle_parser.add_argument("--source", type=Path, required=True)
    bundle_parser.add_argument("--executable", type=Path, required=True)
    bundle_parser.add_argument("--runtime", type=Path, required=True)
    bundle_parser.add_argument("--expected-runtime", type=Path, required=True)
    bundle_parser.add_argument("--output-dir", type=Path, required=True)

    macos_parser = subparsers.add_parser("archive-macos-app")
    macos_parser.add_argument("--version", required=True)
    macos_parser.add_argument("--commit", required=True)
    macos_parser.add_argument("--architecture", required=True)
    macos_parser.add_argument("--executable", type=Path, required=True)
    macos_parser.add_argument("--resource-bundle", type=Path, required=True)
    macos_parser.add_argument("--runtime", type=Path, required=True)
    macos_parser.add_argument("--expected-runtime", type=Path, required=True)
    macos_parser.add_argument("--output-dir", type=Path, required=True)

    icon_parser = subparsers.add_parser("write-windows-icon")
    icon_parser.add_argument("--output", type=Path, required=True)

    windows_parser = subparsers.add_parser("archive-windows-app")
    windows_parser.add_argument("--version", required=True)
    windows_parser.add_argument("--commit", required=True)
    windows_parser.add_argument("--source", type=Path, required=True)
    windows_parser.add_argument("--executable", type=Path, required=True)
    windows_parser.add_argument("--runtime", type=Path, required=True)
    windows_parser.add_argument("--expected-runtime", type=Path, required=True)
    windows_parser.add_argument("--output-dir", type=Path, required=True)

    manifest_parser = subparsers.add_parser("write-manifest")
    manifest_parser.add_argument("--version", required=True)
    manifest_parser.add_argument("--tag", required=True)
    manifest_parser.add_argument("--commit", required=True)
    manifest_parser.add_argument("--assets-dir", type=Path, required=True)
    manifest_parser.add_argument("--output", type=Path, required=True)

    notes_parser = subparsers.add_parser("write-notes")
    notes_parser.add_argument("--version", required=True)
    notes_parser.add_argument("--tag", required=True)
    notes_parser.add_argument("--commit", required=True)
    notes_parser.add_argument("--repository", required=True)
    notes_parser.add_argument("--history", type=Path, required=True)
    notes_parser.add_argument("--since")
    notes_parser.add_argument("--output", type=Path, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "validate":
            validate_identifiers(args.version, args.tag, args.commit)
        elif args.command == "archive-web":
            archive_web(args.version, args.commit, args.source, args.output_dir)
        elif args.command == "archive-native":
            archive_native(
                args.version,
                args.commit,
                args.backend,
                args.source,
                args.output_dir,
            )
        elif args.command == "archive-linux-bundle":
            archive_linux_bundle(
                args.version,
                args.commit,
                args.backend,
                args.source,
                args.executable,
                args.runtime,
                args.expected_runtime,
                args.output_dir,
            )
        elif args.command == "archive-macos-app":
            archive_macos_app(
                args.version,
                args.commit,
                args.architecture,
                args.executable,
                args.resource_bundle,
                args.runtime,
                args.expected_runtime,
                args.output_dir,
            )
        elif args.command == "write-windows-icon":
            write_windows_icon(args.output)
        elif args.command == "archive-windows-app":
            archive_windows_app(
                args.version,
                args.commit,
                args.source,
                args.executable,
                args.runtime,
                args.expected_runtime,
                args.output_dir,
            )
        elif args.command == "write-manifest":
            manifest = build_manifest(
                args.version, args.tag, args.commit, args.assets_dir
            )
            args.output.write_text(
                json.dumps(manifest, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        elif args.command == "write-notes":
            history = json.loads(args.history.read_text(encoding="utf-8"))
            if not isinstance(history, list):
                raise ValueError("GitHub history must be a JSON list")
            notes = render_notes(
                args.version,
                args.tag,
                args.commit,
                args.repository,
                history,
                args.since,
            )
            args.output.write_text(notes, encoding="utf-8")
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"taskapp-release: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
