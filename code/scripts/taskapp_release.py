#!/usr/bin/env python3
"""Validate and assemble incremental TaskApp GitHub release metadata."""

from __future__ import annotations

import argparse
import io
import json
import re
import sys
import tarfile
import zipfile
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
    return {
        "schemaVersion": 1,
        "product": "TaskApp/Trestle",
        "version": version,
        "tag": tag,
        "sourceCommit": commit.lower(),
        "artifacts": artifacts,
        "knownLimitations": [
            "Linux bundles are portable archives, not signed distribution packages.",
            "macOS and Windows remain generated projects; see GitHub issue #13522.",
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
  project for macOS or Windows.

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

`task-app-release-manifest-v{version}.json` records the source commit and exact
verification claim for every payload. `SHA256SUMS` authenticates every payload and
the manifest.

## Known limitations

- Linux payloads are portable archives rather than signed distribution packages.
- macOS and Windows installers are not included yet: [#{13522}]({issue_root}/13522).
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
