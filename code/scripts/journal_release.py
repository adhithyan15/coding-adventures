#!/usr/bin/env python3
"""Validate and assemble Mosaic Journal GitHub release payloads (J5e, #14416).

``.github/workflows/release-journal.yml`` calls this script for every decision
it makes about a release. The workflow builds and launches the apps, and this
script judges what was built. Keeping the judgement here means it can be
unit-tested (``code/scripts/tests/test_journal_release.py``) instead of living
as untested shell inside YAML.

The rule it enforces is the same one ``engram_release.py`` and
``taskapp_release.py`` enforce: **a release only claims payloads that were
verified.** Every archive function checks its input before writing anything, so
an incomplete bundle fails here rather than being published.

What a Journal release is (code/specs/journal-mosaic-app.md, "Release (J5e)")::

    tag      journal-v<semver>          e.g. journal-v0.1.0
    commit   the full 40-hex SHA the dispatch validated
    version  == [package] version in mosaic-package.toml
    payloads journal-web-v<V>.zip            the browser app
             journal-qt-linux-v<V>.zip       Qt 6, Linux x86_64
             journal-compose-linux-v<V>.zip  Compose Desktop, Linux x86_64
             SHA256SUMS                      checksums of the three above

Only the hosts CI actually *launches* ship. SwiftUI only compiles in CI, and
Flutter desktop and XAML have no Journal lane yet, so they are not here. The
spec says what each one is waiting on.

Shared code, not copied code
----------------------------
The archive writer is ``engram_release._zip_tree``. It is the most hardened zip
writer in the repository: sorted members, symlinks stored as links rather than
followed, file modes narrowed, an allowlist of safe path components, refusal of
hard links and case-colliding names, and an atomic rename into place. A second
copy here would drift from it the first time a reviewer hardened one of them.
The strict SemVer and commit patterns come from the same module for the same
reason. Because of that dependency, the release workflow's pull-request path
filter includes ``engram_release.py``.
"""

from __future__ import annotations

import argparse
import datetime as _datetime
import hashlib
import json
import re
import sys
import tomllib
from pathlib import Path

# The sibling module lives beside this one. Importing it by path keeps this
# script runnable as ``python code/scripts/journal_release.py`` from the repo
# root, which is how the workflow calls it, without packaging either file.
sys.path.insert(0, str(Path(__file__).resolve().parent))

from engram_release import COMMIT, SEMVER  # noqa: E402
from engram_release import _reject_root_absolute_assets as reject_root_absolute_assets  # noqa: E402
from engram_release import _zip_tree as zip_tree  # noqa: E402

TAG_PREFIX = "journal-v"

# The runtime the web host fetches, by the name build-web.sh copies it to.
# Named here because its *absence* is the failure that matters: Vite builds a
# page whether or not ``public/`` held the wasm, and that page then fails at
# boot with an empty journal and an error.
WASM_RUNTIME = "journal_mosaic_app.wasm"

# The Linux hosts that ship, and what identifies each one's payload.
#
#   backend   the `mosaic-compile --backend` name
#   label     the middle of the artifact name: journal-<label>-v<V>.zip
#   toolkit   for humans: release notes and INSTALL.txt
#   executable the file name the generated project gives the app. Qt names the
#             binary after the mounted component; Compose's jpackage launcher
#             is named after the package, with every non-alphanumeric
#             character replaced by `_` (compose_gradle_application_id).
#   runtime   the name every generated host loads the Rust engine by
#   needs     the prerequisite a user has to supply
NATIVE_TARGETS: dict[str, dict[str, str]] = {
    "qt": {
        "label": "qt-linux",
        "toolkit": "Qt 6",
        "executable": "JournalApp",
        "runtime": "libmosaic_app.so",
        "needs": "Qt 6.8 libraries installed on a Linux x86_64 system.",
    },
    "compose": {
        "label": "compose-linux",
        "toolkit": "Compose Desktop",
        "executable": "journal_app",
        "runtime": "libmosaic_app.so",
        "needs": "A glibc-based Linux x86_64 system. The app bundles its own JVM.",
    },
}

CHECKSUMS = "SHA256SUMS"


# --------------------------------------------------------------------------
# Identity
# --------------------------------------------------------------------------


def validate_identifiers(version: str, tag: str, commit: str | None = None) -> None:
    """Reject a version, tag or commit that is malformed or does not match.

    A release tag is a permanent public identifier, so this is strict:

        version "0.1.0"   tag "journal-v0.1.0"   accepted
        version "0.1"     (any tag)              not SemVer
        version "0.1.0"   tag "journal-0.1.0"    tag does not match
        commit  "abc123"                         not a full SHA
    """

    if SEMVER.fullmatch(version) is None:
        raise ValueError(f"version is not strict SemVer: {version!r}")
    expected_tag = f"{TAG_PREFIX}{version}"
    if tag != expected_tag:
        raise ValueError(f"tag must be {expected_tag!r}, got {tag!r}")
    if commit is not None and COMMIT.fullmatch(commit) is None:
        raise ValueError("commit must be a full 40-character Git SHA")


def _check_version(version: str) -> None:
    validate_identifiers(version, f"{TAG_PREFIX}{version}")


def manifest_version(manifest: Path) -> str:
    """The `[package] version` a Mosaic package declares."""

    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ValueError(f"cannot read package manifest {manifest}: {error}") from error
    package = data.get("package")
    version = package.get("version") if isinstance(package, dict) else None
    if not isinstance(version, str) or SEMVER.fullmatch(version) is None:
        raise ValueError(f"{manifest} has no strict SemVer [package] version")
    return version


# --------------------------------------------------------------------------
# Changelog
# --------------------------------------------------------------------------
#
# journal-app's changelog is SHARDED. `CHANGELOG.d/_meta.md` holds the title
# and the one section heading. Every other file is one top-level bullet, named
# `NNNNN-SLUG-xxxxxxxx.md`: a five-digit rank (higher is newer; the document
# reads newest first), an uppercase slug, and an eight-hex digest. The renderer
# lives in human-language-data's doc-shard tooling. This only checks what a
# release depends on.
#
#     ## 0.1.0 — unreleased     fine on a pull request, refused for publication
#     ## 0.1.0 — 2026-11-02     fine on a pull request, publishes 0.1.0
#     ## Unreleased             refused: which version?

SECTION_HEADING = re.compile(
    r"^(?P<version>\S+) — (?P<state>unreleased|\d{4}-\d{2}-\d{2})$"
)
SHARD_NAME = re.compile(r"^\d{5}-[A-Z0-9]+(?:-[A-Z0-9]+)*-[0-9a-f]{8}\.md$")


def _section_heading(changelog_dir: Path) -> tuple[str, str]:
    meta = changelog_dir / "_meta.md"
    if not meta.is_file():
        raise ValueError(f"sharded changelog has no entry point: {meta}")
    headings = re.findall(r"^## (.*?)\s*$", meta.read_text(encoding="utf-8"), re.MULTILINE)
    if len(headings) != 1:
        raise ValueError(
            f"{meta} must hold exactly one '## ' section heading, found {len(headings)}"
        )
    match = SECTION_HEADING.fullmatch(headings[0])
    if match is None:
        raise ValueError(
            f"section heading must be '<semver> — unreleased' or "
            f"'<semver> — YYYY-MM-DD', got {headings[0]!r}"
        )
    version, state = match["version"], match["state"]
    if SEMVER.fullmatch(version) is None:
        raise ValueError(f"changelog section version is not strict SemVer: {version!r}")
    if state != "unreleased":
        try:
            _datetime.date.fromisoformat(state)
        except ValueError as error:
            raise ValueError(f"changelog section date is not a real date: {state!r}") from error
    return version, state


def changelog_entries(changelog_dir: Path) -> list[str]:
    """Every shard's text, in document order (highest rank first)."""

    if not changelog_dir.is_dir():
        raise ValueError(f"sharded changelog directory does not exist: {changelog_dir}")
    entries: list[tuple[str, str]] = []
    for path in sorted(changelog_dir.iterdir()):
        if path.name == "_meta.md":
            continue
        if not path.is_file() or SHARD_NAME.fullmatch(path.name) is None:
            raise ValueError(f"not a changelog shard name (NNNNN-SLUG-xxxxxxxx.md): {path.name}")
        text = path.read_text(encoding="utf-8")
        if not text.startswith("- "):
            raise ValueError(f"changelog shard must start with a top-level bullet: {path.name}")
        entries.append((path.name, text.rstrip("\n")))
    if not entries:
        raise ValueError(f"sharded changelog has no entries: {changelog_dir}")
    # Five-digit ranks sort as strings exactly as they sort as numbers.
    entries.sort(key=lambda entry: entry[0], reverse=True)
    return [text for _, text in entries]


def validate_changelog(
    changelog_dir: Path, manifest: Path, release_version: str | None = None
) -> str:
    """Check the changelog a release would publish; return its section version.

    Always: one well-formed section, at the manifest's version, with entries.
    For publication (``release_version`` given): that section is the release
    being published, and it carries a date rather than "unreleased".
    """

    version, state = _section_heading(changelog_dir)
    declared = manifest_version(manifest)
    if version != declared:
        raise ValueError(
            f"changelog section is {version!r} but {manifest.name} declares {declared!r}"
        )
    changelog_entries(changelog_dir)
    if release_version is not None:
        _check_version(release_version)
        if version != release_version:
            raise ValueError(
                f"changelog section is {version!r}, not the release {release_version!r}"
            )
        if state == "unreleased":
            raise ValueError(
                f"changelog section {version!r} is still 'unreleased'; date it "
                f"('## {version} — YYYY-MM-DD') in the release pull request"
            )
    return version


# --------------------------------------------------------------------------
# Payload names
# --------------------------------------------------------------------------


def web_artifact_name(version: str) -> str:
    _check_version(version)
    return f"journal-web-v{version}.zip"


def native_artifact_name(version: str, backend: str) -> str:
    _check_version(version)
    if backend not in NATIVE_TARGETS:
        raise ValueError(
            f"unsupported Journal release backend {backend!r}; "
            f"expected one of {sorted(NATIVE_TARGETS)}"
        )
    return f"journal-{NATIVE_TARGETS[backend]['label']}-v{version}.zip"


def artifact_names(version: str) -> list[str]:
    """The complete payload set of one release, sorted.

    The publish job requires the downloaded files to be exactly this set (plus
    SHA256SUMS), so this function is the one place the set is written down.
    """

    names = [web_artifact_name(version)]
    names.extend(native_artifact_name(version, backend) for backend in NATIVE_TARGETS)
    return sorted(names)


# --------------------------------------------------------------------------
# Verification and archives
# --------------------------------------------------------------------------


def check_native_report(report_path: Path) -> None:
    """Require a strict, native-complete emission.

    ``mosaic-compile --profile native-complete`` writes
    ``mosaic-degradations.json``. A release must not ship a project in which
    any part fell back, so all three must hold:

        nativeComplete          true
        degradations            []
        replacedGeneratedFiles  []    (no hand-written file shadows generated code)
    """

    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read native-complete report {report_path}: {error}") from error
    if not isinstance(report, dict):
        raise ValueError(f"{report_path} is not a JSON object")
    problems = []
    if report.get("nativeComplete") is not True:
        problems.append("nativeComplete is not true")
    if report.get("degradations") != []:
        problems.append(f"degradations is {report.get('degradations')!r}, not []")
    if report.get("replacedGeneratedFiles") != []:
        problems.append(
            f"replacedGeneratedFiles is {report.get('replacedGeneratedFiles')!r}, not []"
        )
    if problems:
        raise ValueError(f"{report_path} is not strict native-complete: {'; '.join(problems)}")


def archive_web(
    version: str, commit: str, source: Path, expected_wasm: Path, output_dir: Path
) -> Path:
    """Verify and archive the production web bundle.

    The bundle must have its entry point, its assets and a non-empty runtime.
    The runtime must be **byte-identical** to the one this job just built, so a
    stale ``public/`` copy cannot ship. ``index.html`` must not reference the
    domain root, because Pages serves the app from a subdirectory.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    missing = [
        str(path)
        for path in (source / "index.html", source / WASM_RUNTIME)
        if not path.is_file()
    ]
    if not (source / "assets").is_dir():
        missing.append(str(source / "assets"))
    if missing:
        raise ValueError(f"web bundle is incomplete: {', '.join(missing)}")
    shipped = (source / WASM_RUNTIME).read_bytes()
    if not shipped:
        raise ValueError(f"{WASM_RUNTIME} is empty")
    if not expected_wasm.is_file() or shipped != expected_wasm.read_bytes():
        raise ValueError(f"{WASM_RUNTIME} in the bundle is not the runtime just built")
    reject_root_absolute_assets(source)
    output = output_dir / web_artifact_name(version)
    zip_tree(source, output, output.name.removesuffix(".zip"), commit.lower())
    return output


def _inside(source: Path, candidate: Path, label: str) -> Path:
    """``candidate``'s path inside ``source``, refusing anything outside it."""

    try:
        resolved = candidate.resolve(strict=True)
    except OSError as error:
        raise ValueError(f"{label} does not exist: {candidate}") from error
    try:
        return resolved.relative_to(source.resolve(strict=True))
    except ValueError as error:
        raise ValueError(f"{label} must be inside the bundle: {candidate}") from error


def install_text(version: str, backend: str, executable: str) -> str:
    """The INSTALL.txt a Linux payload carries."""

    target = NATIVE_TARGETS[backend]
    return f"""Journal {version} — {target['toolkit']}, portable Linux x86_64 bundle

This is an unpack-and-run directory, not a distribution package.
It is not signed.

Needs: {target['needs']}

Run it from anywhere:
  /path/to/journal-{target['label']}-v{version}/{executable}

Journal keeps its entries in one local state file. Set MOSAIC_APP_STATE_PATH
to choose where that file lives. A state file that is not valid JSON is not
deleted: Journal renames it with a `.corrupt` suffix and starts empty.
"""


def archive_native(
    version: str,
    commit: str,
    backend: str,
    source: Path,
    executable: Path,
    runtime: Path,
    expected_runtime: Path,
    output_dir: Path,
) -> Path:
    """Verify and archive one runnable Linux application tree.

    Checked before anything is written:

    * the backend is one that ships;
    * the executable and the runtime are inside ``source``, so the archive
      cannot point at the build machine;
    * the executable has the name the generated project gives it, and is
      executable;
    * the runtime is named ``libmosaic_app.so`` and is byte-identical to the
      release build of ``journal-mosaic-app``. A host that silently bundled a
      stale or different engine would otherwise launch with the wrong one.

    Then an ``INSTALL.txt`` is added at the top of the tree and the tree is
    zipped under one directory named after the file.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    name = native_artifact_name(version, backend)
    target = NATIVE_TARGETS[backend]
    if not source.is_dir():
        raise ValueError(f"bundle source directory does not exist: {source}")
    executable_relative = _inside(source, executable, "executable")
    _inside(source, runtime, "runtime")
    if executable.name != target["executable"]:
        raise ValueError(
            f"{backend} executable must be named {target['executable']!r}, "
            f"got {executable.name!r}"
        )
    if not executable.is_file() or not executable.stat().st_mode & 0o111:
        raise ValueError(f"{backend} executable is not an executable file: {executable}")
    if runtime.name != target["runtime"]:
        raise ValueError(
            f"{backend} runtime must be named {target['runtime']!r}, got {runtime.name!r}"
        )
    if not runtime.is_file() or not expected_runtime.is_file():
        raise ValueError("the bundled and the expected Rust runtime must both exist")
    if runtime.read_bytes() != expected_runtime.read_bytes():
        raise ValueError(
            f"bundled {runtime.name} is not the release build of journal-mosaic-app"
        )
    install = source / "INSTALL.txt"
    if install.exists() or install.is_symlink():
        raise ValueError(f"bundle already has an INSTALL.txt: {install}")
    install.write_text(
        install_text(version, backend, executable_relative.as_posix()), encoding="utf-8"
    )
    output = output_dir / name
    zip_tree(source, output, name.removesuffix(".zip"), commit.lower())
    return output


def write_checksums(version: str, assets_dir: Path) -> Path:
    """Require exactly the declared payloads, then write ``SHA256SUMS``.

    The format is ``sha256sum``'s, so ``sha256sum --check SHA256SUMS`` verifies
    a download. Anything extra or missing is refused: a release must not quietly
    ship more, or less, than it declares.
    """

    expected = artifact_names(version)
    if not assets_dir.is_dir():
        raise ValueError(f"release assets directory does not exist: {assets_dir}")
    actual = sorted(path.name for path in assets_dir.iterdir())
    if actual != expected:
        missing = sorted(set(expected) - set(actual))
        extra = sorted(set(actual) - set(expected))
        raise ValueError(f"release payload mismatch: missing {missing}, unexpected {extra}")
    lines = []
    for name in expected:
        path = assets_dir / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"release payload is not a regular file: {name}")
        digest = hashlib.sha256()
        with path.open("rb") as handle:
            for block in iter(lambda: handle.read(1 << 20), b""):
                digest.update(block)
        lines.append(f"{digest.hexdigest()}  {name}\n")
    output = assets_dir / CHECKSUMS
    output.write_text("".join(lines), encoding="utf-8")
    return output


# --------------------------------------------------------------------------
# Release notes
# --------------------------------------------------------------------------


def render_notes(version: str, commit: str, changelog_dir: Path, manifest: Path) -> str:
    """The release body: the dated changelog section, then what ships.

    Built only from a changelog that validates for publication, so the notes
    can never say "unreleased" or describe a different version.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    validate_changelog(changelog_dir, manifest, version)
    entries = "\n".join(changelog_entries(changelog_dir))
    web = web_artifact_name(version)
    qt = native_artifact_name(version, "qt")
    compose = native_artifact_name(version, "compose")
    return f"""Journal {version}: the Mosaic Journal, in the browser and on Linux.

## Changes

{entries}

## What is in this release

- `{web}`: the browser app. The Rust engine runs as WebAssembly, and entries
  are kept in this browser's local storage. Unzip it and serve the directory
  from any path. The same build is live at /journal/.
- `{qt}`: a Qt 6 app for Linux x86_64. It needs Qt 6.8 libraries installed.
- `{compose}`: a Compose Desktop app for Linux x86_64. It bundles its own
  JVM.
- `SHA256SUMS`: checksums of the three files above. Check a download with
  `sha256sum --check SHA256SUMS`.

Every native payload carries the same `journal-mosaic-app` engine, and the
release lane checks that byte for byte. Each one was launched from its
extracted archive before publication.

## Not in this release

- macOS, Windows, Flutter desktop, iOS and Android. CI builds some of these,
  but a host ships only once CI launches it as it launches these three.
- Signing. Nothing here is signed.

Built from {commit.lower()}.
"""


# --------------------------------------------------------------------------
# Command line
# --------------------------------------------------------------------------


def _cmd_validate(args: argparse.Namespace) -> int:
    validate_identifiers(args.version, args.tag, args.commit)
    print(f"version={args.version}")
    print(f"tag={args.tag}")
    if args.commit:
        print(f"commit={args.commit}")
    return 0


def _cmd_validate_changelog(args: argparse.Namespace) -> int:
    version = validate_changelog(
        Path(args.changelog_dir), Path(args.manifest), args.release_version
    )
    print(f"changelog section {version} is valid")
    return 0


def _cmd_check_native_report(args: argparse.Namespace) -> int:
    check_native_report(Path(args.report))
    print(f"{args.report}: strict native-complete")
    return 0


def _cmd_artifact_names(args: argparse.Namespace) -> int:
    for name in artifact_names(args.version):
        print(name)
    return 0


def _cmd_archive_web(args: argparse.Namespace) -> int:
    print(
        archive_web(
            args.version,
            args.commit,
            Path(args.source),
            Path(args.expected_wasm),
            Path(args.output_dir),
        )
    )
    return 0


def _cmd_archive_native(args: argparse.Namespace) -> int:
    print(
        archive_native(
            args.version,
            args.commit,
            args.backend,
            Path(args.source),
            Path(args.executable),
            Path(args.runtime),
            Path(args.expected_runtime),
            Path(args.output_dir),
        )
    )
    return 0


def _cmd_write_checksums(args: argparse.Namespace) -> int:
    print(write_checksums(args.version, Path(args.assets_dir)))
    return 0


def _cmd_write_notes(args: argparse.Namespace) -> int:
    notes = render_notes(
        args.version, args.commit, Path(args.changelog_dir), Path(args.manifest)
    )
    Path(args.output).write_text(notes, encoding="utf-8")
    print(args.output)
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)

    validate = commands.add_parser("validate", help="Check that version, tag and commit agree")
    validate.add_argument("--version", required=True)
    validate.add_argument("--tag", required=True)
    validate.add_argument("--commit")
    validate.set_defaults(handler=_cmd_validate)

    changelog = commands.add_parser("validate-changelog", help="Check the sharded changelog")
    changelog.add_argument("--changelog-dir", required=True)
    changelog.add_argument("--manifest", required=True)
    changelog.add_argument("--release-version")
    changelog.set_defaults(handler=_cmd_validate_changelog)

    report = commands.add_parser("check-native-report", help="Require a native-complete emission")
    report.add_argument("--report", required=True)
    report.set_defaults(handler=_cmd_check_native_report)

    names = commands.add_parser("artifact-names", help="List the payloads a release publishes")
    names.add_argument("--version", required=True)
    names.set_defaults(handler=_cmd_artifact_names)

    web = commands.add_parser("archive-web", help="Verify and archive the web bundle")
    web.add_argument("--version", required=True)
    web.add_argument("--commit", required=True)
    web.add_argument("--source", required=True)
    web.add_argument("--expected-wasm", required=True)
    web.add_argument("--output-dir", required=True)
    web.set_defaults(handler=_cmd_archive_web)

    native = commands.add_parser("archive-native", help="Verify and archive a Linux app tree")
    native.add_argument("--version", required=True)
    native.add_argument("--commit", required=True)
    native.add_argument("--backend", required=True, choices=sorted(NATIVE_TARGETS))
    native.add_argument("--source", required=True)
    native.add_argument("--executable", required=True)
    native.add_argument("--runtime", required=True)
    native.add_argument("--expected-runtime", required=True)
    native.add_argument("--output-dir", required=True)
    native.set_defaults(handler=_cmd_archive_native)

    checksums = commands.add_parser("write-checksums", help="Check the payload set; write SHA256SUMS")
    checksums.add_argument("--version", required=True)
    checksums.add_argument("--assets-dir", required=True)
    checksums.set_defaults(handler=_cmd_write_checksums)

    notes = commands.add_parser("write-notes", help="Render the release notes")
    notes.add_argument("--version", required=True)
    notes.add_argument("--commit", required=True)
    notes.add_argument("--changelog-dir", required=True)
    notes.add_argument("--manifest", required=True)
    notes.add_argument("--output", required=True)
    notes.set_defaults(handler=_cmd_write_notes)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        return int(args.handler(args))
    except ValueError as error:
        # An unshippable payload is an expected outcome. Report it as one line
        # rather than a traceback, so the CI log says what was wrong.
        print(f"journal_release: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
