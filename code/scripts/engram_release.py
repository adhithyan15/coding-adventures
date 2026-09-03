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
import plistlib
import posixpath
import re
import shutil
import struct
import sys
import tempfile
import unicodedata
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


# The SwiftUI backend, macOS only -- it is a SwiftUI app, so there is nowhere
# else for it to go. Unsigned zip rather than a dmg for the same reason the
# Electron macOS build is: signing needs credentials this project does not have,
# and macOS refuses an unsigned dmg with an error that reads like corruption.
SWIFTUI_TARGETS = {"macos": "zip"}

# Distinct DEFINED `eg_*` symbols that must be present in the packaged binary.
# Five, not the ~47 the cdylib exports: SwiftUI is the one backend that LINKS
# the engine statically, and a static link pulls in only the objects actually
# referenced, so a correct build carries the nine the host calls and nothing
# else. A check written against the full export list would fail on a working
# app.
MIN_LINKED_ENGINE_SYMBOLS = 5

# Mach-O constants, from <mach-o/loader.h> and <mach-o/nlist.h>.
MH_MAGIC_64 = 0xFEEDFACF
MH_CIGAM_64 = 0xCFFAEDFE
FAT_MAGIC = 0xCAFEBABE
FAT_MAGIC_64 = 0xCAFEBABF
LC_SYMTAB = 0x2
N_STAB = 0xE0  # debug entries, which reuse the type field for other meanings
N_TYPE = 0x0E
N_SECT = 0x0E  # defined in a section of THIS file -- the bit that matters
N_UNDF = 0x00  # undefined: expected from somewhere else at load time


def linked_engine_symbols(binary: bytes) -> tuple[set[str], set[str]]:
    """The `eg_*` symbols a Mach-O file defines, and the ones it leaves undefined.

    The symbol TABLE, not a string search. That distinction is the whole point:
    Mach-O stores defined and undefined names identically in the string table,
    so scanning bytes for `eg_...` passes a binary that merely *references* the
    engine and does not contain it -- which is exactly the broken build this
    check exists to catch. Verified against a pair of fixtures compiled both
    ways; a byte scan gave the same verdict for both.

    Parsed by hand rather than shelling out to `nm`, because a toolchain check
    is one that can be absent -- the `nm` assertion in `build-native.sh`
    silently degrades to "unknown" on the Windows runner, and that is how a
    platform ships unverified.
    """

    if len(binary) < 8:
        raise ValueError("file is too short to be a Mach-O binary")

    magic = struct.unpack_from(">I", binary, 0)[0]
    if magic in (FAT_MAGIC, FAT_MAGIC_64):
        # A universal binary: every slice must carry the engine, since we do
        # not know which one the user's machine will run.
        count = struct.unpack_from(">I", binary, 4)[0]
        width = 32 if magic == FAT_MAGIC_64 else 20
        defined: set[str] = set()
        undefined: set[str] = set()
        for index in range(count):
            base = 8 + index * width
            if magic == FAT_MAGIC_64:
                offset, size = struct.unpack_from(">QQ", binary, base + 8)
            else:
                offset, size = struct.unpack_from(">II", binary, base + 8)
            slice_defined, slice_undefined = linked_engine_symbols(
                binary[offset : offset + size]
            )
            defined = slice_defined if index == 0 else defined & slice_defined
            undefined |= slice_undefined
        return defined, undefined

    little = struct.unpack_from("<I", binary, 0)[0]
    if little != MH_MAGIC_64 and magic != MH_CIGAM_64:
        raise ValueError(f"not a 64-bit Mach-O binary (magic {little:#x})")

    ncmds = struct.unpack_from("<I", binary, 16)[0]
    offset = 32  # past mach_header_64
    for _ in range(ncmds):
        cmd, cmdsize = struct.unpack_from("<II", binary, offset)
        if cmd == LC_SYMTAB:
            symoff, nsyms, stroff, strsize = struct.unpack_from(
                "<IIII", binary, offset + 8
            )
            return _read_symtab(binary, symoff, nsyms, stroff, strsize)
        offset += cmdsize
    raise ValueError("Mach-O binary has no symbol table")


def _read_symtab(
    binary: bytes, symoff: int, nsyms: int, stroff: int, strsize: int
) -> tuple[set[str], set[str]]:
    """Walk `nlist_64` entries, splitting `eg_*` names by defined vs undefined."""

    strings = binary[stroff : stroff + strsize]
    defined: set[str] = set()
    undefined: set[str] = set()
    for index in range(nsyms):
        entry = symoff + index * 16
        if entry + 16 > len(binary):
            break
        n_strx, n_type = struct.unpack_from("<IB", binary, entry)
        if n_type & N_STAB:
            continue
        end = strings.find(b"\x00", n_strx)
        if end < 0:
            continue
        name = strings[n_strx:end].decode("utf-8", "replace").lstrip("_")
        if not name.startswith("eg_"):
            continue
        if n_type & N_TYPE == N_SECT:
            defined.add(name)
        elif n_type & N_TYPE == N_UNDF:
            undefined.add(name)
    return defined, undefined


def swiftui_artifact_name(version: str, platform: str) -> str:
    """The published name for the SwiftUI build."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    if platform not in SWIFTUI_TARGETS:
        raise ValueError(f"unknown SwiftUI platform: {platform}")
    return f"engram-swiftui-{platform}-v{version}.{SWIFTUI_TARGETS[platform]}"


# The Flutter desktop platforms. Every one is a zip because `flutter build`
# produces an application DIRECTORY per platform -- a `.app` on macOS, a plain
# bundle tree on Linux, a Release folder on Windows -- rather than an installer.
FLUTTER_TARGETS = {
    "linux": "zip",
    "macos": "zip",
    "windows": "zip",
}

# Where each platform's bundle keeps native libraries. Flutter puts them in a
# different place on every target, which is why the engine's placement is three
# problems rather than one -- and why a check written against one layout would
# pass the other two while shipping an app that cannot open a deck.
FLUTTER_ENGINE_DIRS = {
    "linux": "lib",
    "macos": "Contents/Frameworks",
    "windows": "",  # beside the executable
}


def flutter_artifact_name(version: str, platform: str) -> str:
    """The published name for one platform's Flutter build."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    if platform not in FLUTTER_TARGETS:
        raise ValueError(f"unknown Flutter platform: {platform}")
    return f"engram-flutter-{platform}-v{version}.{FLUTTER_TARGETS[platform]}"


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
    names.extend(
        swiftui_artifact_name(version, platform)
        for platform in sorted(SWIFTUI_TARGETS)
    )
    names.extend(
        flutter_artifact_name(version, platform)
        for platform in sorted(FLUTTER_TARGETS)
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

    Symlinks are **stored as links**, never followed -- see ``_write_symlink``
    for why that is both the secure and the correct behaviour. Everything these
    archives contain is published, so this function refuses a payload rather
    than writing one it cannot vouch for.
    """

    if not source.is_dir():
        raise ValueError(f"archive source directory does not exist: {source}")
    root = source.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)

    # Built beside the target and moved into place only on success. Opening
    # `output` directly would leave a VALID, readable, truncated zip behind when
    # a member is rejected mid-walk -- and a truncated payload satisfies both
    # `if-no-files-found: error` and the publish job's "files on disk equal the
    # declared set" check. Today the workflow's `set -e` stops before the
    # upload; this makes it not depend on that.
    partial = output.with_name(output.name + ".partial")
    try:
        with zipfile.ZipFile(partial, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            provenance = zipfile.ZipInfo(f"{root_name}/SOURCE_COMMIT")
            provenance.create_system = 3
            provenance.external_attr = 0o644 << 16  # else it extracts as 0600
            provenance.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(provenance, f"{commit}\n")
            # Any two members that differ only by case are one file on the
            # reader's macOS or Windows machine, and the second silently
            # overwrites the first. `SOURCE_COMMIT` is the security-relevant
            # instance -- it is seeded here so the general check covers it --
            # but a payload that quietly loses a file on extraction is a bad
            # payload whatever the file was.
            # Keyed exactly as members are, or the seed silently stops
            # matching: `SEMVER` permits an uppercase prerelease, so a tag like
            # `engram-v0.3.0-RC.1` made this entry unreachable.
            seen: dict[str, str] = {
                _collision_key(f"{root_name}/SOURCE_COMMIT"): "SOURCE_COMMIT"
            }
            total = 0
            for path in sorted(source.rglob("*")):
                relative = path.relative_to(source).as_posix()
                _reject_unsafe_member(relative)
                member = f"{root_name}/{relative}"
                clash = seen.setdefault(_collision_key(member), relative)
                if clash != relative:
                    raise ValueError(
                        f"two members collide when case is folded, so one would "
                        f"overwrite the other on extraction: {relative!r} and "
                        f"{clash!r}"
                    )
                if path.is_symlink():
                    _write_symlink(archive, path, root, root_name, member)
                    continue
                if path.is_dir():
                    # Stored explicitly: an empty directory is otherwise
                    # dropped in silence, and a `.app` bundle can need one.
                    archive.writestr(_directory_entry(f"{member}/"), b"")
                    continue

                status = path.stat()
                if not path.is_file():
                    # A FIFO, socket, or device node cannot be published, and
                    # skipping it silently contradicts what this function
                    # promises. Neither a Vite `dist/` nor a jpackage image
                    # contains one, so this only ever fires on a surprise.
                    raise ValueError(
                        f"payload contains a non-regular file: {relative}"
                    )
                if status.st_nlink > 1:
                    # `_write_symlink` refuses to follow a symlink out of the
                    # payload; a hard link is the same reach with none of the
                    # tells -- `is_symlink()` is False and the bytes are simply
                    # embedded. Git cannot store one and neither build produces
                    # one, so refusing costs nothing.
                    raise ValueError(
                        f"payload contains a hard link, which can reach outside "
                        f"it without appearing to: {relative}"
                    )

                total += status.st_size
                if total > MAX_PAYLOAD_BYTES:
                    raise ValueError(
                        f"payload exceeds {MAX_PAYLOAD_BYTES} uncompressed bytes; "
                        f"refusing to publish an archive that expands without "
                        f"bound"
                    )
                _write_file(archive, path, member, status.st_mode)
    except BaseException:
        partial.unlink(missing_ok=True)
        raise
    os.replace(partial, output)


# A path component we are willing to publish. An ALLOWLIST, deliberately.
#
# The alternative -- enumerating unsafe shapes -- is unwinnable, and this file
# learned that the expensive way: three review rounds each found one more
# equivalence rule nobody had modelled. Backslash-as-separator. Case folding.
# Win32 stripping trailing dots and spaces, so `index.html.` and `index.html`
# are two members here and one file on the downloader's machine. Still waiting
# were NTFS `:` alternate data streams, reserved device names, and HFS/APFS
# Unicode normalisation. That rule set is defined by other people's extractors,
# so it can only grow, and every miss is silent.
#
# What we archive, by contrast, is a *closed* set: Vite emits hashed ASCII
# names, and jpackage emits ASCII plus the occasional space. So the safe shapes
# are named instead, and every one of those equivalence rules becomes
# unreachable rather than individually patched. A payload that falls outside
# this fails loudly at release time, which is the right direction to fail; the
# allowlist can then be widened once, with evidence, rather than tightened
# repeatedly under attack.
SAFE_COMPONENT = re.compile(r"[A-Za-z0-9_.][A-Za-z0-9._+ -]*")

# Win32 resolves these as devices no matter the directory or extension.
RESERVED_NAMES = frozenset(
    {"CON", "PRN", "AUX", "NUL"}
    | {f"COM{digit}" for digit in range(1, 10)}
    | {f"LPT{digit}" for digit in range(1, 10)}
)


def _reject_unsafe_member(relative: str) -> None:
    """Refuse a member name we are not certain every extractor reads the same.

    Checked component by component, because the hazards are per-component:
    Win32 strips a trailing dot from each one, and a reserved device name is
    reserved at any depth.
    """

    for component in relative.split("/"):
        if component in {"", ".", ".."}:
            raise ValueError(f"unsafe archive member name: {relative!r}")
        if SAFE_COMPONENT.fullmatch(component) is None:
            raise ValueError(
                f"unsafe archive member name: {relative!r} -- component "
                f"{component!r} is outside the publishable character set"
            )
        # Win32 strips these, so `index.html.` and `index.html` are one file
        # there and two members here: the later one silently wins.
        if component.endswith((".", " ")):
            raise ValueError(
                f"unsafe archive member name: {relative!r} -- component "
                f"{component!r} ends with a dot or space, which Windows strips"
            )
        if component.split(".")[0].upper() in RESERVED_NAMES:
            raise ValueError(
                f"unsafe archive member name: {relative!r} -- component "
                f"{component!r} is a reserved Windows device name"
            )

    # Case-folded, because the collision is. On a case-sensitive build
    # filesystem `source_commit` and `SOURCE_COMMIT` are two distinct members
    # and nothing warns -- the shadowing then happens on the *downloader's*
    # machine, where macOS and Windows collapse them and the planted value
    # wins. Nested `sub/SOURCE_COMMIT` is still fine: it does not collide.
    if _collision_key(relative) == _collision_key("SOURCE_COMMIT"):
        raise ValueError(
            "payload contains its own SOURCE_COMMIT, which would shadow the "
            "commit recorded here"
        )


def _collision_key(relative: str) -> str:
    """How the reader's filesystem will decide two members are one file.

    Case folding is the familiar half. Normalisation is the other: macOS
    case-insensitive volumes are normalisation-insensitive too, so a name in
    NFD and the same name in NFC are one file there and two members in an
    archive built on Linux. The allowlist above already confines names to
    ASCII, where both rules are trivial -- this stays explicit so the guarantee
    does not quietly depend on that.
    """

    return unicodedata.normalize("NFC", relative).casefold()


def _reject_unsafe_target(path: Path, target: str) -> None:
    """Refuse a symlink target we cannot vouch for on the reader's machine.

    The same shapes as a member name, for the same reasons -- these were
    written separately once and the target half was missing the checks
    entirely, which is why they now share ``SAFE_COMPONENT``. A backslash is
    the sharp one: `posixpath` treats it as an ordinary character, so a target
    of ``..\\..\\..\\evil.exe`` normalises to a single harmless-looking
    in-payload component under the rules we validate with, and splits to
    ``../evil.exe`` under the rules that extract it.
    """

    if not target:
        raise ValueError(f"unsafe symlink target: {path} -> empty")
    for component in target.split("/"):
        if component in {"", "."}:
            continue
        if component == "..":
            continue  # Where it lands is checked below, not here.
        if SAFE_COMPONENT.fullmatch(component) is None or component.endswith(
            (".", " ")
        ):
            raise ValueError(
                f"unsafe symlink target: {path} -> {target!r} -- component "
                f"{component!r} is outside the publishable character set"
            )


# A ceiling on what one release payload may expand to. The Compose
# distribution bundles a JDK runtime and is the largest thing here by far, at a
# few hundred megabytes; this sits well above that so it only ever fires on
# something absurd. Without it, one planted zero-filled file compresses at
# ~1000:1 and the published asset is a decompression bomb aimed at downloaders.
MAX_PAYLOAD_BYTES = 4 * 1024 * 1024 * 1024


def _directory_entry(member: str) -> zipfile.ZipInfo:
    """An explicit directory member, so empty directories survive."""

    info = zipfile.ZipInfo(member)
    info.create_system = 3
    info.external_attr = (0o040755 << 16) | 0x10
    return info


def _write_file(
    archive: zipfile.ZipFile, path: Path, member: str, mode: int
) -> None:
    """Copy one regular file in, with its mode narrowed to what we publish.

    The executable bit is carried through -- an app that loses it extracts
    unlaunchable -- but setuid, setgid, and group/other write are dropped
    rather than published. Streamed rather than read whole: the JDK runtime in
    a Compose distribution contains individual files of a hundred megabytes.
    """

    info = zipfile.ZipInfo.from_file(path, member)
    info.compress_type = zipfile.ZIP_DEFLATED
    info.create_system = 3
    permissions = 0o755 if mode & 0o111 else 0o644
    info.external_attr = permissions << 16
    with path.open("rb") as source, archive.open(info, "w") as target:
        shutil.copyfileobj(source, target)


# Deep enough for any real bundle -- a jpackage framework chain is two or
# three hops -- and shallow enough that a pathological chain fails fast.
MAX_SYMLINK_HOPS = 40


def _reject_symlink_loop(path: Path) -> None:
    """Refuse a symlink that never reaches a real file.

    Walked by hand because ``resolve()`` does not agree with itself across
    platforms: a loop raises ``RuntimeError`` on macOS and is silently
    tolerated on Linux. Left to it, the same payload would be refused on the
    macOS runner and published from the Linux one -- and a check whose verdict
    depends on where it ran is worse than no check, because it looks like one.

    A *dangling* link is fine and stays accepted: the loop below stops as soon
    as it reaches something that is not a symlink, which includes nothing at
    all. jpackage bundles legitimately contain dangling links.
    """

    seen: set[str] = set()
    probe = path
    for _ in range(MAX_SYMLINK_HOPS):
        try:
            if not probe.is_symlink():
                return
        except OSError:
            return
        key = str(probe)
        if key in seen:
            raise ValueError(f"symlink loop, which never reaches a file: {path}")
        seen.add(key)
        probe = probe.parent / os.readlink(probe)
    raise ValueError(f"symlink chain longer than {MAX_SYMLINK_HOPS} hops: {path}")


def _write_symlink(
    archive: zipfile.ZipFile, path: Path, root: Path, root_name: str, member: str
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

    _reject_unsafe_target(path, target)

    # An absolute target cannot be right in a relocatable payload: it either
    # dangles on the reader's machine or points somewhere unrelated, and it
    # leaks the runner's filesystem layout into a public download.
    if posixpath.isabs(target) or os.path.isabs(target):
        raise ValueError(f"symlink target is absolute: {path} -> {target}")

    # Checked twice, against two different roots, because they catch different
    # things. The build-tree check below asks where the link points *now*. This
    # one asks where it will point *after extraction*, which is what the reader
    # actually gets -- and a link can satisfy the first while violating the
    # second: from `dist/assets/`, `../../dist/evil` lands back inside the
    # build tree, but from `<payload>/assets/` it escapes the payload root.
    # This is a NECESSARY but NOT SUFFICIENT filter, and the physical
    # `resolve()` below is load-bearing -- do not "simplify" by dropping it.
    # `rglob` never descends into a symlink, so every component *above* the
    # link is a real directory; the TARGET can still traverse one, and
    # `normpath` collapses `L/..` to L's parent rather than L's target's
    # parent. `legit/../../evil` is judged in-payload here and caught there.
    landing = posixpath.normpath(posixpath.join(posixpath.dirname(member), target))
    if landing != root_name and not landing.startswith(f"{root_name}/"):
        raise ValueError(f"symlink escapes the payload: {path} -> {target}")

    # Detected explicitly rather than left to `resolve()`, which disagrees
    # across platforms: a loop raises `RuntimeError` on macOS and is silently
    # tolerated on Linux, so the same payload would be refused on one runner
    # and published on the other. A loop is never legitimate here.
    _reject_symlink_loop(path)

    try:
        resolved = (path.parent / target).resolve()
    except (OSError, RuntimeError) as error:
        raise ValueError(f"cannot resolve symlink: {path} -> {target}") from error
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


def archive_swiftui(
    version: str, platform: str, source: Path, output_dir: Path, commit: str
) -> Path:
    """Archive the SwiftUI `.app` bundle for publication.

    The engine check differs from every other backend's, because SwiftUI is the
    one that links `engram-capi` statically instead of loading it at runtime.
    There is no library file to look for beside the binary -- the engine either
    is inside the executable or the app launches into a UI where every deck
    operation silently does nothing.

    So the packaged executable is scanned for engine symbol *names*, by reading
    its bytes. Deliberately not `nm`: a toolchain check is one that can be
    absent, and this repository has already shipped a platform unverified
    exactly that way -- the `nm` assertion in `build-native.sh` silently
    degrades to "unknown" on the Windows runner. Reading bytes works wherever
    Python does, so it cannot skip.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if not source.is_dir():
        raise ValueError(f"SwiftUI bundle does not exist: {source}")
    if source.suffix != ".app":
        raise ValueError(f"expected a .app bundle, got: {source.name}")

    info_plist = source / "Contents" / "Info.plist"
    if not info_plist.is_file():
        # Without it macOS does not treat the directory as an application at
        # all: it opens as a folder rather than launching.
        raise ValueError(f"bundle has no Info.plist: {source}")

    # By NAME, from the plist. Taking the first entry of a sorted glob instead
    # verifies whichever file happens to sort first -- so a bundle carrying a
    # second file in `Contents/MacOS` would have that one scanned while the
    # executable macOS actually launches shipped unexamined. Both get published.
    try:
        declared = plistlib.loads(info_plist.read_bytes()).get("CFBundleExecutable")
    except Exception as error:  # noqa: BLE001 - any malformed plist is a refusal
        raise ValueError(f"bundle has an unreadable Info.plist: {error}") from error
    if not declared or "/" in declared or declared in {".", ".."}:
        raise ValueError(f"bundle has no usable CFBundleExecutable: {source}")

    macos = source / "Contents" / "MacOS"
    if macos.is_symlink() or not macos.is_dir():
        raise ValueError(f"Contents/MacOS is not a real directory: {source}")
    present = sorted(path.name for path in macos.iterdir())
    if present != [declared]:
        raise ValueError(
            f"Contents/MacOS must hold exactly {declared!r}, found {present}"
        )

    binary = macos / declared
    if binary.is_symlink() or not binary.is_file():
        raise ValueError(f"bundle executable is not a regular file: {binary}")

    defined, undefined = linked_engine_symbols(binary.read_bytes())
    if len(defined) < MIN_LINKED_ENGINE_SYMBOLS:
        raise ValueError(
            f"only {len(defined)} DEFINED engine symbols in {binary.name}; the "
            f"engine did not link, so the app would launch with every deck "
            f"operation silently unavailable "
            f"(defined: {', '.join(sorted(defined)) or 'none'}; "
            f"undefined: {', '.join(sorted(undefined)) or 'none'})"
        )
    if undefined:
        raise ValueError(
            f"{binary.name} leaves engine symbols undefined, so it expects them "
            f"from a library that will not be there: {', '.join(sorted(undefined))}"
        )

    name = swiftui_artifact_name(version, platform)
    output = output_dir / name

    # Staged rather than archiving `source.parent`, which is the SwiftPM
    # project directory: that shipped `.build/` too -- module caches, object
    # files, the dSYM, and the static archive -- turning a 3.8 MB app into a
    # 47 MB download of build litter. The app ran perfectly, so nothing but
    # reading the member list would have caught it.
    with tempfile.TemporaryDirectory() as staging:
        # Before the copy: `copytree` dereferences hard links, so `_zip_tree`'s
        # hard-link guard can never fire on a staged tree. See
        # `_reject_hard_links`.
        _reject_hard_links(source)

        staged = Path(staging) / source.name
        shutil.copytree(source, staged, symlinks=True)

        # `_zip_tree` confines symlinks to the payload ROOT, which after
        # staging is the directory *above* the bundle -- so a link out of the
        # `.app` and into that directory would satisfy it, and a bundle whose
        # `Contents/MacOS` pointed sideways could ship with the verified binary
        # not in the archive at all. Containment is re-asserted here against
        # the bundle itself, which is the boundary that actually matters.
        _reject_links_out_of(staged)

        _zip_tree(
            Path(staging), output, f"engram-swiftui-{platform}-v{version}", commit
        )
    return output


def _reject_hard_links(tree: Path) -> None:
    """Refuse hard links, before staging destroys the evidence.

    `_zip_tree` already refuses them -- a hard link reaches outside the payload
    with none of a symlink's tells -- but that guard cannot fire for anything
    archived through a staging copy, because `shutil.copytree` DEREFERENCES a
    hard link: it reads the bytes and writes a fresh file with `st_nlink == 1`.
    By the time the archiver walks the staged tree there is nothing left to
    detect.

    Demonstrated on a fixture: a bundle hard-linking a file outside it is
    refused by `_zip_tree` directly, and the same tree staged and archived
    produced a member holding that file's contents verbatim. These are public
    release assets, and `.git/config` on a runner holds the checkout token.
    """

    for path in tree.rglob("*"):
        if path.is_symlink() or not path.is_file():
            continue
        if path.stat().st_nlink > 1:
            raise ValueError(
                f"payload contains a hard link, which can reach outside it "
                f"without appearing to: {path.relative_to(tree)}"
            )


def _reject_links_out_of(bundle: Path) -> None:
    """Refuse any symlink inside ``bundle`` that resolves outside it."""

    root = bundle.resolve()
    for path in bundle.rglob("*"):
        if not path.is_symlink():
            continue
        target = os.readlink(path)
        landing = (path.parent / target).resolve()
        if landing != root and not landing.is_relative_to(root):
            raise ValueError(f"symlink escapes the bundle: {path} -> {target}")


def _cmd_archive_swiftui(args: argparse.Namespace) -> int:
    output = archive_swiftui(
        args.version,
        args.platform,
        Path(args.source),
        Path(args.output_dir),
        args.commit,
    )
    print(output)
    return 0


def archive_flutter(
    version: str, platform: str, source: Path, output_dir: Path, commit: str
) -> Path:
    """Archive a Flutter desktop bundle for publication.

    Flutter resolves `engram-capi` at RUNTIME from the bundle, like Qt and
    Compose and unlike SwiftUI, so the check here is that the library file is
    present -- and present in the directory THIS platform's loader looks in.
    `flutter build` succeeds whether or not it is there, and an app missing it
    launches perfectly with every deck operation silently unavailable.

    The per-platform layout is the trap. `Contents/Frameworks` on macOS, `lib`
    on Linux, beside the executable on Windows: a check written against one of
    them passes the other two while shipping a broken app, which is precisely
    the failure this repository has already shipped once with Compose.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if platform not in FLUTTER_TARGETS:
        raise ValueError(f"unknown Flutter platform: {platform}")
    if not source.is_dir():
        raise ValueError(f"Flutter bundle does not exist: {source}")

    expected_dir = source / FLUTTER_ENGINE_DIRS[platform] if FLUTTER_ENGINE_DIRS[
        platform
    ] else source
    engine = _find_engine(expected_dir, platform)
    if engine is None:
        # Named separately from "not in the bundle at all", because the two
        # have different causes: a missing engine is a build that skipped the
        # copy, while one in the wrong place is a layout assumption that has
        # drifted from what this platform's loader actually reads.
        elsewhere = _find_engine(source, platform, recursive=True)
        if elsewhere is not None:
            raise ValueError(
                f"the engine is in the bundle but not where {platform} looks "
                f"for it: found {elsewhere.relative_to(source)}, expected it "
                f"under {FLUTTER_ENGINE_DIRS[platform] or '(the bundle root)'}"
            )
        raise ValueError(
            f"Flutter {platform} bundle has no engram_capi engine: {source}"
        )
    if engine.stat().st_size == 0:
        raise ValueError(f"engram_capi engine is empty: {engine}")

    name = flutter_artifact_name(version, platform)
    output = output_dir / name
    with tempfile.TemporaryDirectory() as staging:
        _reject_hard_links(source)
        staged = Path(staging) / source.name
        shutil.copytree(source, staged, symlinks=True)
        _reject_links_out_of(staged)

        # Re-asserted against the tree about to be zipped, not the one that was
        # inspected a moment ago. Everything above holds of `source`; what gets
        # published is `staged`, and verifying one while shipping the other is
        # how a check stops being about the artifact.
        staged_dir = (
            staged / FLUTTER_ENGINE_DIRS[platform]
            if FLUTTER_ENGINE_DIRS[platform]
            else staged
        )
        staged_engine = _find_engine(staged_dir, platform)
        if staged_engine is None or staged_engine.stat().st_size == 0:
            raise ValueError(
                f"the staged bundle has no usable engine at "
                f"{FLUTTER_ENGINE_DIRS[platform] or '(the bundle root)'}"
            )

        _zip_tree(
            Path(staging), output, f"engram-flutter-{platform}-v{version}", commit
        )
    return output


# What a real shared library starts with, per platform. Checked because the
# earlier form matched on FILENAME alone, which accepts a `.pdb`, a one-byte
# file, or a text file that happens to be called `engram_capi` -- and a Rust
# cdylib on Windows emits `engram_capi.dll`, `engram_capi.dll.lib` and
# `engram_capi.pdb` side by side, so a copy step with a sloppy glob can pick up
# the debug symbols and pass a name check while shipping no engine at all.
#
# This proves the file is a shared library of the right kind for its platform,
# in the directory that platform's loader reads. It does NOT prove the library
# exports the engine's symbols; saying more than that in the release notes
# would be describing a guarantee the code does not make.
LIBRARY_MAGIC = {
    "linux": ((b"\x7fELF",), (".so",)),
    "macos": ((b"\xcf\xfa\xed\xfe", b"\xca\xfe\xba\xbe", b"\xce\xfa\xed\xfe"), (".dylib",)),
    "windows": ((b"MZ",), (".dll",)),
}


def _find_engine(
    directory: Path, platform: str, *, recursive: bool = False
) -> Path | None:
    """The `engram_capi` shared library in ``directory``, if there is one."""

    if not directory.is_dir():
        return None
    magics, suffixes = LIBRARY_MAGIC[platform]
    candidates = directory.rglob("*") if recursive else directory.iterdir()
    for path in sorted(candidates):
        if not path.is_file() or path.is_symlink():
            continue
        # Matched on the NAME, not `stem`: `stem` strips one suffix, so a real
        # versioned soname like `libengram_capi.so.0.4.0` would not match, and
        # `engram_capi.pdb` would.
        name = path.name.removeprefix("lib")
        if not any(
            name == f"engram_capi{suffix}" or name.startswith(f"engram_capi{suffix}.")
            for suffix in suffixes
        ):
            continue
        with path.open("rb") as handle:
            head = handle.read(4)
        if any(head.startswith(magic) for magic in magics):
            return path
    return None


def _cmd_archive_flutter(args: argparse.Namespace) -> int:
    output = archive_flutter(
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

    archive_flutter_cmd = subcommands.add_parser(
        "archive-flutter", help="Archive a Flutter desktop bundle for publication"
    )
    archive_flutter_cmd.add_argument("--version", required=True)
    archive_flutter_cmd.add_argument("--platform", required=True)
    archive_flutter_cmd.add_argument("--source", required=True)
    archive_flutter_cmd.add_argument("--output-dir", required=True)
    archive_flutter_cmd.add_argument("--commit", required=True)
    archive_flutter_cmd.set_defaults(handler=_cmd_archive_flutter)

    archive_swiftui_cmd = subcommands.add_parser(
        "archive-swiftui", help="Archive the SwiftUI .app bundle for publication"
    )
    archive_swiftui_cmd.add_argument("--version", required=True)
    archive_swiftui_cmd.add_argument("--platform", required=True)
    archive_swiftui_cmd.add_argument("--source", required=True)
    archive_swiftui_cmd.add_argument("--output-dir", required=True)
    archive_swiftui_cmd.add_argument("--commit", required=True)
    archive_swiftui_cmd.set_defaults(handler=_cmd_archive_swiftui)

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
