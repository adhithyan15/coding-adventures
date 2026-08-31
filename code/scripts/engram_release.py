#!/usr/bin/env python3
"""Validate and assemble Engram GitHub release payloads.

Engram v0.3.0 ships the **web** build of the Mosaic app plus the Rust core.
Native installers follow in a later tag, once the native lanes verify them; see
the release issue for that decision.

The rule this file exists to enforce is that a release only ever claims
artifacts that were actually verified. Every archive function checks the payload
it is given before writing anything, so an incomplete bundle fails here rather
than being published and discovered later.

Modelled on ``taskapp_release.py``, deliberately: the two products should not
diverge in how they validate identifiers or shape their payloads.
"""

from __future__ import annotations

import argparse
import re
import sys
import zipfile
from pathlib import Path

TAG_PREFIX = "engram-v"

# Strict SemVer, from semver.org's own reference expression. Deliberately not a
# loose `\d+\.\d+\.\d+`: a release tag is a permanent public identifier, and
# "1.2.3.4" or "01.2.3" should be rejected at the door rather than published.
SEMVER = re.compile(
    r"^(0|[1-9]\d*)\."
    r"(0|[1-9]\d*)\."
    r"(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)
COMMIT = re.compile(r"^[0-9a-fA-F]{40}$")

# The engine the browser build loads. Named here because its *absence* is the
# failure this module most needs to catch: the app builds, loads, and runs
# without it, and only fails when a user tries to import a deck.
WASM_ENGINE = "engram_engine.wasm"


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
    """Every payload this release publishes.

    One entry today. It is a list rather than a string so that adding native
    bundles later does not change the shape of the contract, and so the workflow
    can assert the set it uploaded matches the set declared here.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    return [f"engram-web-v{version}.zip"]


def _zip_tree(source: Path, output: Path, root_name: str, commit: str) -> None:
    """Archive ``source`` under a single top-level directory.

    Members are written in sorted order so the archive is reproducible: the same
    tree produces the same bytes regardless of filesystem iteration order. The
    ``SOURCE_COMMIT`` member records exactly which commit produced the payload,
    so an artifact found later can be traced without relying on the release page.
    """

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
    """Verify and archive the production web bundle.

    The completeness check is the point. A Vite build succeeds whether or not
    ``public/`` contained the engine, so a bundle missing ``engram_engine.wasm``
    is a *runtime* failure hiding behind a green build — the user gets a working
    app that cannot import a deck. That is precisely the shape of the bug that
    shipped once already as a stale committed artifact, so it is checked here
    rather than assumed.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)

    missing: list[str] = []
    if not (source / "index.html").is_file():
        missing.append(str(source / "index.html"))
    if not (source / WASM_ENGINE).is_file():
        missing.append(str(source / WASM_ENGINE))
    if not (source / "assets").is_dir():
        missing.append(str(source / "assets"))
    if missing:
        raise ValueError(f"web bundle is incomplete: {', '.join(missing)}")

    # An engine that is present but empty would satisfy the check above while
    # failing at load. Cheap to rule out; expensive to diagnose in the wild.
    if (source / WASM_ENGINE).stat().st_size == 0:
        raise ValueError(f"{WASM_ENGINE} is empty")

    output = output_dir / f"engram-web-v{version}.zip"
    _zip_tree(source, output, f"engram-web-v{version}", commit)
    return output


def _cmd_validate(args: argparse.Namespace) -> int:
    validate_identifiers(args.version, args.tag, args.commit)
    print(f"version={args.version}")
    print(f"tag={args.tag}")
    if args.commit:
        print(f"commit={args.commit}")
    return 0


def _cmd_artifact_names(args: argparse.Namespace) -> int:
    for name in artifact_names(args.version):
        print(name)
    return 0


def _cmd_archive_web(args: argparse.Namespace) -> int:
    output = archive_web(
        args.version, args.commit, Path(args.source), Path(args.output_dir)
    )
    print(output)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)

    validate = subcommands.add_parser(
        "validate", help="Check that a version, tag, and commit agree"
    )
    validate.add_argument("--version", required=True)
    validate.add_argument("--tag", required=True)
    validate.add_argument("--commit")
    validate.set_defaults(handler=_cmd_validate)

    names = subcommands.add_parser(
        "artifact-names", help="List the payloads this release publishes"
    )
    names.add_argument("--version", required=True)
    names.set_defaults(handler=_cmd_artifact_names)

    web = subcommands.add_parser(
        "archive-web", help="Verify and archive the production web bundle"
    )
    web.add_argument("--version", required=True)
    web.add_argument("--commit", required=True)
    web.add_argument("--source", required=True)
    web.add_argument("--output-dir", required=True)
    web.set_defaults(handler=_cmd_archive_web)

    args = parser.parse_args(argv)
    try:
        return int(args.handler(args))
    except ValueError as error:
        # A release payload problem is a normal, expected outcome here — report
        # it as a message rather than a traceback so the CI log is readable.
        print(f"engram_release: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
