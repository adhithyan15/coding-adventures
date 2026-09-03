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


# More Mach-O load commands, from <mach-o/loader.h>. `LC_REQ_DYLD` is set on
# commands the loader must understand, and is part of the stored value.
LC_REQ_DYLD = 0x80000000
LC_LOAD_DYLIB = 0x0C
LC_LOAD_WEAK_DYLIB = 0x18 | LC_REQ_DYLD
LC_REEXPORT_DYLIB = 0x1F | LC_REQ_DYLD
DYLIB_COMMANDS = (LC_LOAD_DYLIB, LC_LOAD_WEAK_DYLIB, LC_REEXPORT_DYLIB)
LC_RPATH = 0x1C | LC_REQ_DYLD

# Prefixes that make a dependency relocatable: resolved relative to the binary
# or the bundle, so the app works wherever the user unzipped it.
RELOCATABLE_PREFIXES = ("@executable_path", "@loader_path", "@rpath")

# Absolute paths that are fine because they are part of macOS itself and are
# present on every machine.
SYSTEM_LIBRARY_PREFIXES = ("/usr/lib/", "/System/Library/")


# A fat binary has a handful of slices. A header claiming more, or slices that
# do not lie inside the file, is malformed -- and left unchecked it is a
# resource attack: each recursion holds a live copy of its slice, so a chain of
# fat headers turned a 2 MB file into 2 GB of RSS before hitting Python's
# recursion limit. A nested fat header is never legitimate.
MAX_FAT_SLICES = 32


def _fat_slices(binary: bytes) -> list[bytes] | None:
    """The slices of a universal binary, or ``None`` if this is not one."""

    if len(binary) < 8:
        return None
    magic = struct.unpack_from(">I", binary, 0)[0]
    if magic not in (FAT_MAGIC, FAT_MAGIC_64):
        return None

    count = struct.unpack_from(">I", binary, 4)[0]
    if count > MAX_FAT_SLICES:
        raise ValueError(f"implausible fat architecture count: {count}")
    width = 32 if magic == FAT_MAGIC_64 else 20

    slices: list[bytes] = []
    for index in range(count):
        base = 8 + index * width
        if base + width > len(binary):
            raise ValueError("fat architecture table runs past the file")
        if magic == FAT_MAGIC_64:
            offset, size = struct.unpack_from(">QQ", binary, base + 8)
        else:
            offset, size = struct.unpack_from(">II", binary, base + 8)
        if offset < 8 or size < 32 or offset + size > len(binary):
            raise ValueError("fat architecture entry lies outside the file")
        chunk = binary[offset : offset + size]
        if struct.unpack_from(">I", chunk, 0)[0] in (FAT_MAGIC, FAT_MAGIC_64):
            raise ValueError("nested fat header")
        slices.append(chunk)
    return slices


def _load_commands(binary: bytes):
    """Yield ``(cmd, offset, cmdsize)`` for each load command, or refuse.

    Every bound is checked. `cmdsize` of zero never advances the cursor, and
    `ncmds` is read straight from the header, so an unvalidated walk can be
    stalled by its own input -- 50 million iterations measured on a 64-byte
    file. A short `cmdsize` also makes the parser read the NEXT command's bytes
    as a name offset.
    """

    if len(binary) < 32:
        raise ValueError("file is too short to be a Mach-O binary")
    if struct.unpack_from("<I", binary, 0)[0] != MH_MAGIC_64:
        raise ValueError("not a 64-bit little-endian Mach-O binary")

    ncmds, sizeofcmds = struct.unpack_from("<II", binary, 16)
    offset = 32
    end = min(len(binary), 32 + sizeofcmds)
    try:
        for _ in range(ncmds):
            if offset + 8 > end:
                break
            cmd, cmdsize = struct.unpack_from("<II", binary, offset)
            if cmdsize < 8 or offset + cmdsize > end:
                raise ValueError(f"malformed load command at offset {offset}")
            yield cmd, offset, cmdsize
            offset += cmdsize
    except struct.error as error:
        # Otherwise a truncated table raises `struct.error`, which `main` does
        # not catch -- a traceback where the module's one-line refusal belongs.
        raise ValueError(f"truncated Mach-O load commands: {error}") from error


def dylib_dependencies(binary: bytes) -> list[str]:
    """Every dynamic library a Mach-O file asks the loader for.

    Read from the load commands rather than by running `otool`, for the same
    reason the symbol table is: a toolchain check is one that can be absent.
    """

    slices = _fat_slices(binary)
    if slices is not None:
        paths: list[str] = []
        for chunk in slices:
            for path in dylib_dependencies(chunk):
                if path not in paths:
                    paths.append(path)
        return paths

    paths = []
    for cmd, offset, cmdsize in _load_commands(binary):
        if cmd not in DYLIB_COMMANDS:
            continue
        name_offset = struct.unpack_from("<I", binary, offset + 8)[0]
        if name_offset < 8 or name_offset >= cmdsize:
            raise ValueError(f"dylib name offset outside its command: {name_offset}")
        start = offset + name_offset
        end = binary.find(b"\x00", start, offset + cmdsize)
        if end > start:
            paths.append(binary[start:end].decode("utf-8", "replace"))
    return paths


def load_rpaths(binary: bytes) -> list[str]:
    """The `LC_RPATH` entries `@rpath` is resolved against.

    Parsed because without them `@rpath/QtCore.framework/...` says nothing: an
    `@rpath` dependency plus an `LC_RPATH` of `/opt/homebrew/opt/qtbase/lib` is
    a machine-local binary wearing a relocatable-looking install name.
    """

    slices = _fat_slices(binary)
    if slices is not None:
        found: list[str] = []
        for chunk in slices:
            for path in load_rpaths(chunk):
                if path not in found:
                    found.append(path)
        return found

    found = []
    for cmd, offset, cmdsize in _load_commands(binary):
        if cmd != LC_RPATH:
            continue
        path_offset = struct.unpack_from("<I", binary, offset + 8)[0]
        if path_offset < 8 or path_offset >= cmdsize:
            raise ValueError("rpath offset outside its command")
        start = offset + path_offset
        end = binary.find(b"\x00", start, offset + cmdsize)
        if end > start:
            found.append(binary[start:end].decode("utf-8", "replace"))
    return found


def non_relocatable_dependencies(
    binary: bytes, *, depth_in_bundle: int = 0
) -> list[str]:
    """The dependencies that would not resolve on someone else's machine.

    This is THE check for a Qt payload. A Qt app builds and runs perfectly on
    the machine that built it while linking its frameworks by absolute path --
    `/opt/homebrew/opt/qtbase/lib/QtCore.framework/...` -- so the artifact is
    broken for every user who does not happen to have Qt installed at that
    exact path, and nothing about the build says so. `macdeployqt` copies the
    frameworks in and rewrites these paths; this asserts that it actually did.

    Paths are NORMALISED and the prefix must be followed by a separator, so
    `@executable_path/../../../../opt/homebrew/...` and
    `/usr/lib/../../opt/homebrew/...` are refused -- both were accepted by a
    plain `startswith`, which made this a spelling check rather than a
    resolution one. `@executable_pathological/evil.dylib` was accepted too.

    ``depth_in_bundle`` is how many directories separate this binary from the
    bundle root, and it is what makes `..` judgeable rather than guessed at.
    The standard macdeployqt install name is
    `@executable_path/../Frameworks/QtCore.framework/QtCore`: from
    `Contents/MacOS` that climbs one level and lands INSIDE the bundle. A rule
    that treated any `..` as an escape rejected the correct, universal shape --
    caught here only by running it against a real deployed bundle, which is the
    difference between a check and a check that can ship.
    """

    bad: list[str] = []

    # An `@rpath` dependency is only as relocatable as the run paths it
    # resolves against, so an `LC_RPATH` pointing outside the bundle matters --
    # but ONLY if something actually resolves through it.
    #
    # This distinction was not theoretical. Applied unconditionally, the check
    # refused the real deployed bundle over
    # `LC_RPATH /opt/homebrew/Cellar/dbus/1.16.2_1/lib` in `libdbus-1.3.dylib`.
    # Measuring the bundle settled it: 4 binaries carry an outside run path and
    # NONE of them has a single `@rpath` dependency, so dyld never consults it
    # -- which is why the bundle launches. Rejecting there would have failed
    # every Qt release for a path that does nothing.
    #
    # It is still leaked build-machine detail in a public artifact, but that is
    # not what this function is for, and refusing a working payload to tidy it
    # would be the wrong trade.
    rpath_dependencies = [
        path for path in dylib_dependencies(binary) if path.startswith("@rpath")
    ]
    if rpath_dependencies:
        run_paths = load_rpaths(binary)
        inside = [
            rpath
            for rpath in run_paths
            if any(
                rpath == prefix or rpath.startswith(prefix + "/")
                for prefix in ("@executable_path", "@loader_path")
            )
        ]
        if not inside:
            bad.extend(
                f"@rpath dependency {path} with no in-bundle LC_RPATH "
                f"(run paths: {', '.join(run_paths) or 'none'})"
                for path in rpath_dependencies
            )

    for path in dylib_dependencies(binary):
        matched = False
        for prefix in RELOCATABLE_PREFIXES:
            if path == prefix or path.startswith(prefix + "/"):
                matched = True
                rest = posixpath.normpath(path[len(prefix) :].lstrip("/"))
                hops = 0
                while rest.startswith("../") or rest == "..":
                    hops += 1
                    rest = rest[3:] if rest.startswith("../") else ""
                if hops > depth_in_bundle:
                    bad.append(path)
                break
        if matched:
            continue
        normalised = posixpath.normpath(path)
        if not normalised.startswith(SYSTEM_LIBRARY_PREFIXES):
            bad.append(path)
    return bad


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


# The Qt backend. macOS only FOR NOW, and the reason is deployment tooling
# rather than the app: `macdeployqt` ships with Qt and makes a bundle
# relocatable, `windeployqt` does the same on Windows, and Linux has no
# official equivalent -- so each platform is its own piece of work. Declaring
# only what is actually verified keeps the publish job's set-equality check
# honest; see the follow-up issues.
QT_TARGETS = {"macos": "zip"}

LC_CODE_SIGNATURE = 0x1D

# Thin 64-bit little-endian, and both universal spellings.
MACH_O_MAGICS = (b"\xcf\xfa\xed\xfe", b"\xca\xfe\xba\xbe", b"\xca\xfe\xba\xbf")


def qt_artifact_name(version: str, platform: str) -> str:
    """The published name for one platform's Qt build."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    if platform not in QT_TARGETS:
        raise ValueError(f"unknown Qt platform: {platform}")
    return f"engram-qt-{platform}-v{version}.{QT_TARGETS[platform]}"


# The XAML / WinUI 3 backend. Windows only, and not as a scoping decision:
# WinUI's markup compiler is a Windows-native tool, so `dotnet` restores and
# type-checks elsewhere and then stops. There is nowhere else for this to run.
#
# NOT YET IN `artifact_names`, and deliberately so. `--backend xaml --build`
# cannot currently compile this package at all: the emitter writes an event
# type for BOTH layout variants into one namespace, and C# rejects the
# duplicate (#14230). A `build-xaml` job that cannot pass would sit in the
# publish job's `needs` and block every release, so the archiver below is
# finished and tested but unwired. Adding the payload here and the job to the
# workflow is the whole remaining step once #14230 lands.
XAML_TARGETS = {"windows": "zip"}


# The number of `eg_*` exports a real engine DLL carries. The same floor
# `build-native.sh` applies on Linux and macOS -- and the reason this exists is
# that the shell check has no Windows arm at all: its `case "$(uname -s)"` falls
# through to `*) EXPORTS="unknown"`, so on the ONE platform where nobody can
# launch the payload by hand, the engine's contract was never verified.
MIN_EXPORTED_ENGINE_SYMBOLS = 20


def pe_exported_names(binary: bytes) -> list[str]:
    """The names a PE image exports.

    Parsed from the export directory rather than trusted from the filename,
    because `MZ` is not a discriminator: every PE starts with it, including
    the app's own managed assembly. Copying `EngramApp.dll` over
    `engram_capi.dll` passed every check this lane had -- an app that launches
    into a UI where nothing works, on the platform with no local verification.

    Returns an empty list for a PE with no export directory, which is the
    normal shape of a managed assembly or an apphost.
    """

    if len(binary) < 0x40 or binary[:2] != b"MZ":
        raise ValueError("not a PE image")
    pe_offset = struct.unpack_from("<I", binary, 0x3C)[0]
    if pe_offset + 24 > len(binary) or binary[pe_offset : pe_offset + 4] != b"PE\x00\x00":
        raise ValueError("missing PE signature")

    number_of_sections = struct.unpack_from("<H", binary, pe_offset + 6)[0]
    optional_size = struct.unpack_from("<H", binary, pe_offset + 20)[0]
    optional = pe_offset + 24
    if optional + 2 > len(binary):
        raise ValueError("truncated optional header")
    magic = struct.unpack_from("<H", binary, optional)[0]
    if magic == 0x20B:  # PE32+
        directories = optional + 112
    elif magic == 0x10B:  # PE32
        directories = optional + 96
    else:
        raise ValueError(f"unknown optional header magic {magic:#x}")
    if directories + 8 > len(binary) or optional_size < (directories - optional) + 8:
        return []

    export_rva, export_size = struct.unpack_from("<II", binary, directories)
    if export_rva == 0 or export_size == 0:
        return []

    sections = []
    section_table = optional + optional_size
    for index in range(min(number_of_sections, 96)):
        entry = section_table + index * 40
        if entry + 40 > len(binary):
            break
        virtual_address, raw_size, raw_pointer = struct.unpack_from(
            "<III", binary, entry + 12
        )
        sections.append((virtual_address, raw_size, raw_pointer))

    def to_offset(rva: int) -> int | None:
        for virtual_address, raw_size, raw_pointer in sections:
            if virtual_address <= rva < virtual_address + raw_size:
                return raw_pointer + (rva - virtual_address)
        return None

    table = to_offset(export_rva)
    if table is None or table + 40 > len(binary):
        return []
    # IMAGE_EXPORT_DIRECTORY: NumberOfNames at +24, AddressOfNames at +32.
    # Reading +28 gets AddressOfFunctions -- an array of CODE addresses -- and
    # the "names" come back as disassembly. Caught only by running this against
    # real exporting DLLs; a fixture built from the same wrong layout would
    # have agreed with the parser exactly.
    name_count = struct.unpack_from("<I", binary, table + 24)[0]
    names_rva = struct.unpack_from("<I", binary, table + 32)[0]
    names_offset = to_offset(names_rva)
    if names_offset is None:
        return []

    names: list[str] = []
    for index in range(min(name_count, 65536)):
        entry = names_offset + index * 4
        if entry + 4 > len(binary):
            break
        name_rva = struct.unpack_from("<I", binary, entry)[0]
        start = to_offset(name_rva)
        if start is None:
            continue
        end = binary.find(b"\x00", start)
        if end > start:
            names.append(binary[start:end].decode("utf-8", "replace"))
    return names


def xaml_artifact_name(version: str, platform: str) -> str:
    """The published name for the XAML build."""

    validate_identifiers(version, f"{TAG_PREFIX}{version}")
    if platform not in XAML_TARGETS:
        raise ValueError(f"unknown XAML platform: {platform}")
    return f"engram-xaml-{platform}-v{version}.{XAML_TARGETS[platform]}"


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
    names.extend(
        qt_artifact_name(version, platform) for platform in sorted(QT_TARGETS)
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
#
# `@` and a leading `+` were added that way. The Qt payload refused to archive
# on Qt's own QML resources, and the measurement had to be redone before it was
# right: the first pass counted CHARACTERS outside the set and reported only
# `@` (Apple retina naming, `close_big@2x.png`, 566 times). It missed
# `+Fusion`, `+Material`, `+Imagine`, `+Universal` -- QML file-selector
# directories -- because `+` was already legal in the body and the violation
# was the LEADING-character rule. Measuring the failing COMPONENTS instead of
# the offending characters gave the real population: 4 distinct names across
# 17,689 components, with the Flutter and SwiftUI payloads clean.
#
# Both characters are safe where they were added: neither is a separator, a
# Windows-reserved character, or a metacharacter to any extractor. A leading
# `-` stays rejected, because that is the one that reads as an option.
SAFE_COMPONENT = re.compile(r"[A-Za-z0-9_.+@][A-Za-z0-9._+@ -]*")

# Win32 resolves these as devices no matter the directory or extension.
RESERVED_NAMES = frozenset(
    {"CON", "PRN", "AUX", "NUL"}
    | {f"COM{digit}" for digit in range(0, 10)}
    | {f"LPT{digit}" for digit in range(0, 10)}
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
        if component.split(".")[0].rstrip(" ").upper() in RESERVED_NAMES:
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


def is_code_signed(binary: bytes) -> bool:
    """Whether a Mach-O file carries a code-signature load command.

    Presence, not validity -- validity needs `codesign`, which exists on the
    macOS runner and is checked there at build time. This catches the case that
    actually happens: `macdeployqt` rewrites install names, which INVALIDATES
    every signature it touched, and on arm64 the loader then refuses the
    dylibs and the app exits immediately with no output whatsoever.

    Worth having as a separate assertion because the relocatability check
    passes on that broken bundle. Every path is relocatable and the app still
    does not start; they are two different claims.
    """

    if len(binary) < 32:
        return False
    slices = _fat_slices(binary)
    if slices is not None:
        # EVERY slice, not "it is fat so assume so". The earlier form returned
        # True for a fat header followed by zeros -- and a Qt build for
        # `arm64;x86_64` is universal, which is the normal shape for a public
        # macOS release, so this gate would have silently stopped asserting
        # anything the moment the build went universal.
        return bool(slices) and all(is_code_signed(chunk) for chunk in slices)

    if len(binary) < 32 or struct.unpack_from("<I", binary, 0)[0] != MH_MAGIC_64:
        return False
    return any(cmd == LC_CODE_SIGNATURE for cmd, _, _ in _load_commands(binary))


def archive_qt(
    version: str, platform: str, source: Path, output_dir: Path, commit: str
) -> Path:
    """Archive the Qt `.app` bundle for publication.

    Three separate claims are checked, because a Qt payload can fail each one
    while satisfying the others:

    1. **It carries the engine.** Qt resolves `engram-capi` at runtime via
       `QDir(appDir).filePath(...)`, and for a bundled app `appDir` is
       `Contents/MacOS` -- so the engine goes beside the executable, not into
       `Frameworks`.
    2. **It is relocatable.** `qt_add_executable` links the frameworks by
       ABSOLUTE path, so an undeployed binary runs perfectly for whoever built
       it and fails to launch for everyone else. This is the check that makes a
       Qt release worth publishing at all.
    3. **It is signed.** `macdeployqt` invalidates signatures when it rewrites
       install names, and on arm64 the loader refuses such a dylib -- the app
       exits instantly with no output. Verified the hard way: a bundle that
       passed check 2 completely still would not start.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if platform not in QT_TARGETS:
        raise ValueError(f"unknown Qt platform: {platform}")
    if not source.is_dir() or source.suffix != ".app":
        raise ValueError(f"expected a .app bundle, got: {source}")

    info_plist = source / "Contents" / "Info.plist"
    if not info_plist.is_file():
        raise ValueError(f"bundle has no Info.plist: {source}")
    try:
        declared = plistlib.loads(info_plist.read_bytes()).get("CFBundleExecutable")
    except Exception as error:  # noqa: BLE001 - any malformed plist is a refusal
        raise ValueError(f"bundle has an unreadable Info.plist: {error}") from error
    if not declared or "/" in declared:
        raise ValueError(f"bundle has no usable CFBundleExecutable: {source}")

    macos_dir = source / "Contents" / "MacOS"
    if macos_dir.is_symlink() or not macos_dir.is_dir():
        raise ValueError(f"Contents/MacOS is not a real directory: {source}")

    binary = macos_dir / declared
    if binary.is_symlink() or not binary.is_file():
        raise ValueError(f"bundle executable is not a regular file: {binary}")

    engine = _find_engine(macos_dir, platform)
    if engine is None:
        raise ValueError(
            "the Qt bundle has no engine beside its executable in "
            "Contents/MacOS; the app would launch with every deck operation "
            "silently unavailable"
        )

    # EVERY Mach-O in the bundle, not just the executable. The failure being
    # guarded against is that `macdeployqt` rewrites the DYLIBS' install names
    # and invalidates their signatures -- so checking only the top-level binary
    # examines the one file least likely to be wrong, while a bundled framework
    # still pointing at `/opt/homebrew/opt/qtbase/...` passes every gate and
    # fails to launch for every downloader. This bundle holds 159 Mach-O files.
    checked = 0
    for candidate in sorted(source.rglob("*")):
        if candidate.is_symlink() or not candidate.is_file():
            continue
        with candidate.open("rb") as handle:
            head = handle.read(4)
        if head not in MACH_O_MAGICS:
            continue
        blob = candidate.read_bytes()
        where = candidate.relative_to(source)
        # How far this binary sits from the bundle root, which bounds how many
        # `..` hops can still land inside it.
        stray = non_relocatable_dependencies(
            blob, depth_in_bundle=len(where.parent.parts)
        )
        if stray:
            raise ValueError(
                f"{where} links {len(stray)} librar(ies) by a path that will "
                f"not resolve elsewhere, so the app would not launch on a "
                f"machine without them: {', '.join(stray)}"
            )
        if not is_code_signed(blob):
            raise ValueError(
                f"{where} carries no code signature; macdeployqt invalidates "
                f"signatures when it rewrites install names, and on arm64 the "
                f"loader refuses such a binary -- the app exits with no output"
            )
        checked += 1
    if checked == 0:
        raise ValueError(f"the Qt bundle contains no Mach-O binaries: {source}")

    name = qt_artifact_name(version, platform)
    output = output_dir / name
    with tempfile.TemporaryDirectory() as staging:
        _reject_hard_links(source)
        staged = Path(staging) / source.name
        shutil.copytree(source, staged, symlinks=True)
        _reject_links_out_of(staged)
        _zip_tree(Path(staging), output, f"engram-qt-{platform}-v{version}", commit)
    return output


def _cmd_archive_qt(args: argparse.Namespace) -> int:
    output = archive_qt(
        args.version,
        args.platform,
        Path(args.source),
        Path(args.output_dir),
        args.commit,
    )
    print(output)
    return 0


def archive_xaml(
    version: str, platform: str, source: Path, output_dir: Path, commit: str
) -> Path:
    """Archive the XAML / WinUI publish output for publication.

    `dotnet publish` writes a plain directory rather than a bundle, and .NET
    probes for native libraries BESIDE the executable -- so the engine belongs
    at the root of that directory, which is also where `build-native.sh` puts
    it. Checked again here because packaging is a second chance to lose it, and
    the Compose backend lost it exactly that way once.

    Note on verification: this is the one backend that cannot be built or run
    on a developer machine that is not Windows, so unlike the other four the
    payload was never launched by hand before shipping. The checks below are
    correspondingly structural, and the CI job asserts the same things against
    the real publish output.
    """

    validate_identifiers(version, f"{TAG_PREFIX}{version}", commit)
    if platform not in XAML_TARGETS:
        raise ValueError(f"unknown XAML platform: {platform}")
    if not source.is_dir():
        raise ValueError(f"XAML publish output does not exist: {source}")

    # The apphost is identified by NAME, derived from the managed assembly that
    # has a matching runtimeconfig -- not by "some file ends in .exe". Any
    # `.exe` satisfied the earlier check, so a publish output with the apphost
    # missing and only `createdump.exe` present shipped a zip with nothing
    # launchable in it, which is exactly the shape `UseAppHost=false` produces.
    stems = [
        path.stem
        for path in sorted(source.glob("*.runtimeconfig.json"))
        if (source / f"{path.stem.removesuffix('.runtimeconfig')}.dll").is_file()
    ]
    stems = [stem.removesuffix(".runtimeconfig") for stem in stems]
    if not stems:
        raise ValueError(
            f"no managed assembly with a matching runtimeconfig.json at the "
            f"root of {source}; the apphost would fail to start"
        )
    stem = stems[0]

    apphost = source / f"{stem}.exe"
    if not apphost.is_file() or apphost.read_bytes()[:2] != b"MZ":
        raise ValueError(
            f"the publish output has no {stem}.exe apphost at its root; "
            f"nothing in the payload would be launchable"
        )
    for required in (f"{stem}.deps.json", f"{stem}.runtimeconfig.json"):
        if not (source / required).is_file():
            raise ValueError(
                f"the publish output is missing {required}, which the apphost "
                f"treats as a fatal startup error"
            )

    engine = _find_engine(source, "windows")
    if engine is None:
        raise ValueError(
            f"the XAML publish output has no engram_capi.dll beside its "
            f"executable; .NET probes there, so the app would launch with "
            f"every deck operation silently unavailable: {source}"
        )

    # The engine's CONTRACT, not just a file with the right name. `MZ` is not a
    # discriminator -- every PE starts with it, including this app's own
    # managed assembly -- so copying `EngramApp.dll` over `engram_capi.dll`
    # passed every check this lane had. `build-native.sh` gates the other
    # platforms on `nm | grep -c eg_`, but its `case "$(uname -s)"` has no
    # Windows arm and falls through to "unknown", so on the ONE platform where
    # nobody can launch the payload by hand, nothing verified the engine at all.
    exported = [name for name in pe_exported_names(engine.read_bytes()) if name.startswith("eg_")]
    if len(exported) < MIN_EXPORTED_ENGINE_SYMBOLS:
        raise ValueError(
            f"{engine.name} exports only {len(exported)} eg_* symbols; the host "
            f"resolves ~40, and a library that exists but exports nothing "
            f"produces the same silent, feature-free app as no library at all"
        )

    # `FlattenNativeRuntimeDlls` copies the WindowsAppSDK natives out of
    # `runtimes/win-x64/native/` and beside the executable, because the
    # unpackaged bootstrap looks for them there. That target runs
    # `AfterTargets="Build"` into `$(OutDir)`, and this lane archives
    # `$(PublishDir)` -- a different directory. The csproj guards its `.pri`
    # and `.xbf` against exactly this and never guarded the natives.
    natives = source / "runtimes" / "win-x64" / "native"
    if natives.is_dir():
        missing = sorted(
            path.name
            for path in natives.glob("*.dll")
            if not (source / path.name).is_file()
        )
        if missing:
            raise ValueError(
                f"{len(missing)} native runtime librar(ies) are only under "
                f"runtimes/win-x64/native and not beside the executable, where "
                f"the unpackaged bootstrap looks: {', '.join(missing[:5])}"
            )

    name = xaml_artifact_name(version, platform)
    output = output_dir / name
    with tempfile.TemporaryDirectory() as staging:
        _reject_hard_links(source)
        # Named for the app, not `publish` -- the directory name is what a
        # downloader sees after extracting, and every other backend nests
        # something meaningful there.
        staged = Path(staging) / "Engram"
        shutil.copytree(source, staged, symlinks=True)
        _reject_links_out_of(staged)
        if _find_engine(staged, "windows") is None:
            raise ValueError("the staged publish output lost its engine")
        _zip_tree(Path(staging), output, f"engram-xaml-{platform}-v{version}", commit)
    return output


def _cmd_archive_xaml(args: argparse.Namespace) -> int:
    output = archive_xaml(
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

    archive_xaml_cmd = subcommands.add_parser(
        "archive-xaml", help="Archive the XAML publish output for publication"
    )
    archive_xaml_cmd.add_argument("--version", required=True)
    archive_xaml_cmd.add_argument("--platform", required=True)
    archive_xaml_cmd.add_argument("--source", required=True)
    archive_xaml_cmd.add_argument("--output-dir", required=True)
    archive_xaml_cmd.add_argument("--commit", required=True)
    archive_xaml_cmd.set_defaults(handler=_cmd_archive_xaml)

    archive_qt_cmd = subcommands.add_parser(
        "archive-qt", help="Archive the Qt .app bundle for publication"
    )
    archive_qt_cmd.add_argument("--version", required=True)
    archive_qt_cmd.add_argument("--platform", required=True)
    archive_qt_cmd.add_argument("--source", required=True)
    archive_qt_cmd.add_argument("--output-dir", required=True)
    archive_qt_cmd.add_argument("--commit", required=True)
    archive_qt_cmd.set_defaults(handler=_cmd_archive_qt)

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
