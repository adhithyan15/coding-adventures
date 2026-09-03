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
import os
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


# The desktop platforms, and the file extension electron-builder produces for
# each. macOS is a plain zip rather than a dmg because signing and notarisation
# need credentials this build does not have, and an unsigned dmg is worse than a
# zip: macOS refuses to open it with an error that reads like corruption.
DESKTOP_TARGETS = {
    "linux": "AppImage",
    "macos": "zip",
    "windows": "exe",
}


# The Compose Desktop platforms. Every one is a zip because
# `createDistributable` produces an application DIRECTORY -- a `.app` bundle on
# macOS, a plain tree elsewhere -- rather than a single installer file.
#
# Electron is not the point of shipping these. The whole argument for Mosaic is
# that one declarative package yields real native apps on every platform, and a
# release carrying only an Electron build has demonstrated nothing a web bundle
# could not. Compose is the cheapest breadth of the five native backends: it
# runs on the JVM, so one job definition covers all three platforms.
COMPOSE_TARGETS = {
    "linux": "zip",
    "macos": "zip",
    "windows": "zip",
}


def compose_artifact_name(version: str, platform: str) -> str:
    """The published name for one platform's Compose Desktop build."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    if platform not in COMPOSE_TARGETS:
        raise ValueError(f"unknown Compose platform: {platform}")
    return f"engram-compose-{platform}-v{version}.{COMPOSE_TARGETS[platform]}"


def artifact_names(version: str) -> list[str]:
    """Every payload this release publishes.

    The workflow asserts the set on disk equals this set, so a job that silently
    produced nothing cannot result in a release that quietly ships less than it
    claims. That check is only meaningful if this list is the single place the
    payload set is written down.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    names = [f"engram-web-v{version}.zip"]
    names.extend(
        f"engram-desktop-{platform}-v{version}.{extension}"
        for platform, extension in sorted(DESKTOP_TARGETS.items())
    )
    names.extend(
        compose_artifact_name(version, platform)
        for platform in sorted(COMPOSE_TARGETS)
    )
    return names


def desktop_artifact_name(version: str, platform: str) -> str:
    """The published name for one platform's desktop build."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    if platform not in DESKTOP_TARGETS:
        raise ValueError(
            f"unknown desktop platform {platform!r}; "
            f"expected one of {sorted(DESKTOP_TARGETS)}"
        )
    return f"engram-desktop-{platform}-v{version}.{DESKTOP_TARGETS[platform]}"


def _zip_tree(source: Path, output: Path, root_name: str, commit: str) -> None:
    """Archive ``source`` under a single top-level directory.

    Members are written in sorted order so the archive is reproducible: the same
    tree produces the same bytes regardless of filesystem iteration order. The
    ``SOURCE_COMMIT`` member records exactly which commit produced the payload,
    so an artifact found later can be traced without relying on the release page.
    """

    if not source.is_dir():
        raise ValueError(f"archive source directory does not exist: {source}")
    root = source.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr(f"{root_name}/SOURCE_COMMIT", f"{commit}\n")
        for path in sorted(source.rglob("*")):
            relative = path.relative_to(source).as_posix()
            _reject_unsafe_member(relative)
            if path.is_symlink():
                _write_symlink(archive, path, root, f"{root_name}/{relative}")
            elif path.is_file():
                archive.write(path, f"{root_name}/{relative}")


def _reject_unsafe_member(relative: str) -> None:
    """Refuse a member name that some extractor would read as a path.

    ``relative_to`` cannot produce ``..`` or a leading slash, so the classic
    zip-slip shapes are already impossible. A backslash is the one that gets
    through: it is a perfectly legal character in a POSIX filename and
    ``as_posix`` passes it along, but several third-party Windows extractors
    treat it as a separator, so a file named ``..\\..\\evil.exe`` would land
    outside the target directory.
    """

    if "\\" in relative or any(ord(ch) < 0x20 for ch in relative):
        raise ValueError(f"unsafe archive member name: {relative!r}")


def _write_symlink(
    archive: zipfile.ZipFile, path: Path, root: Path, member: str
) -> None:
    """Store a symlink as a link, and only if it stays inside the payload.

    Storing rather than following is not a detail. ``ZipFile.write`` opens the
    path, so a symlink is archived as *its target's bytes* under an innocuous
    in-tree name -- and these archives are published as public release assets.
    Anything able to drop one symlink into the build tree could have the
    contents of any runner-readable file (``.git/config`` holds the checkout
    token) baked into a download, with a member name that looks ordinary. The
    ``zip -qry`` this replaced passed ``-y``, which stores links, so following
    them would have been a regression as well as a leak.

    Storing is also the only correct behaviour for the payload: a macOS
    ``.app`` from jpackage carries a bundled runtime full of symlinked
    directories, and ``rglob`` does not descend into those -- so dereferencing
    silently *drops* every file beneath them while appearing to succeed.

    A link pointing outside the distribution is refused rather than stored:
    inside the payload it would dangle at best, and it is the shape an
    exfiltration attempt takes.
    """

    target = os.readlink(path)
    resolved = (path.parent / target).resolve()
    if resolved != root and not resolved.is_relative_to(root):
        raise ValueError(f"symlink escapes the payload: {path} -> {target}")

    info = zipfile.ZipInfo(member)
    info.create_system = 3  # Unix, so the mode below is read back as a mode.
    info.external_attr = 0o120777 << 16  # S_IFLNK | 0777
    archive.writestr(info, target)


# `src="/assets/…"`, `href="/assets/…"`, and the bare engine path. Matching on
# the quote plus leading slash is what distinguishes a root-absolute URL from a
# relative one (`./assets/…`) or a protocol-relative/external one (`//host/…`).
ROOT_ABSOLUTE_REF = re.compile(r'(?:src|href)\s*=\s*"(/(?!/)[^"]*)"')


def _reject_root_absolute_assets(source: Path) -> None:
    """Refuse a bundle whose entry point can only be served from a domain root.

    This is the check that the v0.3.0 bundle needed and did not have. Every
    other check here asks whether a file is *present*; this one asks whether the
    references between them *resolve*. A bundle can pass all of the former and
    still be broken, because `index.html` returns 200 from any path while its
    script 404s — the page renders blank, which looks far more like a working
    deploy than a failure.

    Only `index.html` is scanned, deliberately. It is the entry point, so if its
    references are relative the bundle relocates; hashed JS chunks contain
    minified string literals where a leading slash is often not a URL at all,
    and matching those would trade a real check for a noisy one.
    """

    index = source / "index.html"
    offenders = sorted(set(ROOT_ABSOLUTE_REF.findall(index.read_text(encoding="utf-8"))))
    if offenders:
        raise ValueError(
            "index.html references assets from the domain root "
            f"({', '.join(offenders)}), so the bundle only works when served "
            "from the root of a domain — unzip it into a subdirectory and the "
            "page loads blank. Emit with a relative Vite `base`."
        )


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

    _reject_root_absolute_assets(source)

    output = output_dir / f"engram-web-v{version}.zip"
    _zip_tree(source, output, f"engram-web-v{version}", commit)
    return output


def archive_compose(
    version: str, platform: str, source: Path, output_dir: Path, commit: str
) -> Path:
    """Archive a Compose Desktop distribution for publication.

    Python's `zipfile` rather than the `zip` command, because `zip` does not
    exist in Git Bash on the Windows runner -- the Compose app built there
    perfectly and then failed to package, which is a silly way to lose a
    platform. Python is already set up in every one of these jobs, so this is
    the one tool that behaves identically on all three.

    `ZipFile.write` carries each file's mode into `external_attr`, so the
    executable bits in the distribution survive the round trip. An archive that
    loses them extracts to an app that cannot be launched.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if not source.is_dir():
        raise ValueError(f"Compose distribution does not exist: {source}")

    # Same reasoning as the wasm check in `archive_web`, and the same bug this
    # repository has already shipped once: Gradle's `createDistributable`
    # succeeds whether or not the engine was copied in beside the jar, so a
    # distribution missing `engram_capi` is a *runtime* failure behind a green
    # build -- an app that launches and then cannot open a deck.
    #
    # Checked here rather than in `build-native.sh` because the shell script's
    # symbol assertion runs `nm`, which does not exist on the Windows runner
    # and is skipped there. This path runs identically on all three.
    engine = next(
        (
            path
            for path in source.rglob("*")
            if not path.is_symlink()
            and path.is_file()
            and path.stem.removeprefix("lib") == "engram_capi"
        ),
        None,
    )
    if engine is None:
        raise ValueError(
            f"Compose distribution has no engram_capi engine: {source}"
        )
    if engine.stat().st_size == 0:
        raise ValueError(f"engram_capi engine is empty: {engine}")

    name = compose_artifact_name(version, platform)
    output = output_dir / name
    _zip_tree(source, output, f"engram-compose-{platform}-v{version}", commit)
    return output


def _cmd_archive_compose(args: argparse.Namespace) -> int:
    output = archive_compose(
        args.version,
        args.platform,
        Path(args.source),
        Path(args.output_dir),
        args.commit,
    )
    print(output)
    return 0


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


def _cmd_compose_name(args: argparse.Namespace) -> int:
    print(compose_artifact_name(args.version, args.platform))
    return 0


def _cmd_desktop_name(args: argparse.Namespace) -> int:
    print(desktop_artifact_name(args.version, args.platform))
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

    archive_compose_cmd = subcommands.add_parser(
        "archive-compose", help="Archive a Compose distribution for publication"
    )
    archive_compose_cmd.add_argument("--version", required=True)
    archive_compose_cmd.add_argument("--platform", required=True)
    archive_compose_cmd.add_argument("--source", required=True)
    archive_compose_cmd.add_argument("--output-dir", required=True)
    archive_compose_cmd.add_argument("--commit", required=True)
    archive_compose_cmd.set_defaults(handler=_cmd_archive_compose)

    compose = subcommands.add_parser(
        "compose-name", help="The published name for one platform's Compose build"
    )
    compose.add_argument("--version", required=True)
    compose.add_argument("--platform", required=True)
    compose.set_defaults(handler=_cmd_compose_name)

    desktop = subcommands.add_parser(
        "desktop-name", help="The published name for one platform's desktop build"
    )
    desktop.add_argument("--version", required=True)
    desktop.add_argument("--platform", required=True, choices=sorted(DESKTOP_TARGETS))
    desktop.set_defaults(handler=_cmd_desktop_name)

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
