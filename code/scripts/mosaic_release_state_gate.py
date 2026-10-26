#!/usr/bin/env python3
"""The shared Mosaic release upgrade and corrupt-recovery gate.

Specification: ``code/specs/mosaic-release-state-gate-v1.md``.

The problem, in one paragraph
=============================

Every Mosaic desktop app persists its state the same way. The generated host
asks the Rust runtime for a ``Snapshot`` and writes it to one per-user file::

    {"bytes":[123,34,100,...],"schema":"engram-mosaic-app","version":3}

``schema`` and ``version`` say what shape the payload has; ``bytes`` *is* the
payload, in serde's encoding of a ``Vec<u8>`` -- one JSON integer per byte. A
release that cannot read the previous release's file throws the user's data
away on the first launch after an upgrade. A host that rejects a file must not
destroy it either: it renames it to ``<state>.corrupt`` and starts empty. This
module proves both properties against a real extracted bundle, for any app.

The three halves
================

::

    committed fixture ──materialize──▶ live state file ──(launch new build)──┐
                                                                            │
                     ┌──────────────────────────────────────────────────────┘
                     ▼
              verify: no .corrupt, schema same, version accepted,
                      bytes valid, pointers equal expected values

    garbage ──▶ live state file + probe copy ──(launch new build)──▶
              verify-quarantine: .corrupt == probe, byte for byte

The module is standard-library only on purpose: it runs on every release
runner -- Linux, macOS and Windows -- before anything is installed.

Safety posture
==============

The gate reads files that a just-built, untrusted-until-proven binary wrote.
So it never follows a symlink at a state path, refuses anything that is not a
regular file, caps every read, checks every byte value, and resolves JSON
pointers by plain dictionary and list indexing. Nothing from a fixture or a
state file is ever evaluated.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import stat
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# --------------------------------------------------------------------------
# Constants
# --------------------------------------------------------------------------

FIXTURE_SCHEMA = "mosaic/release-upgrade-fixture"
FIXTURE_VERSION = 1

# Fixture-seeded state is tiny -- Engram's v0.4.0 fixture is under 500 bytes of
# payload, TaskApp's under 5 KB. The caps exist so a runaway host (or a wrong
# path pointing at something huge) fails fast instead of exhausting a runner's
# memory: each payload byte costs up to four characters in the envelope
# (``255,``) and a Python ``int`` object once parsed.
MAX_STATE_BYTES = 16 * 1024 * 1024
MAX_FIXTURE_BYTES = 4 * 1024 * 1024

# Strict SemVer 2.0.0, the reference expression from semver.org -- the same one
# ``taskapp_release.py`` and ``engram_release.py`` use, so a fixture's source
# version is held to the rule that made it a release in the first place.
SEMVER = re.compile(
    r"^(0|[1-9]\d*)\."
    r"(0|[1-9]\d*)\."
    r"(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)

# A product identifier: what goes in front of every error message.
APP_ID = re.compile(r"^[a-z0-9][a-z0-9-]*$")

# RFC 6901 array index: ``0`` or a decimal with no leading zero. ``01`` and
# ``-`` (the past-the-end marker, meaningful only for JSON Patch "add") never
# name an existing element, so they never resolve.
ARRAY_INDEX = re.compile(r"^(0|[1-9][0-9]*)$")

FIXTURE_KEYS_REQUIRED = frozenset(
    {
        "fixtureSchema",
        "fixtureVersion",
        "sourceApp",
        "sourceVersion",
        "snapshotSchema",
        "snapshotVersion",
        "acceptedLiveVersions",
        "expectations",
        "state",
    }
)
FIXTURE_KEYS_OPTIONAL = frozenset({"provenance"})
EXPECTATION_KEYS_REQUIRED = frozenset({"pointer", "equals"})
EXPECTATION_KEYS_OPTIONAL = frozenset({"liveVersions"})


class GateError(ValueError):
    """A gate check failed. The message says which, and for which app."""


# --------------------------------------------------------------------------
# The in-memory fixture
# --------------------------------------------------------------------------
#
# Files of the generic schema are parsed into these. They are also the seam an
# app with a legacy fixture plugs into: ``taskapp_release.py`` builds a
# ``GateFixture`` from its own older file shape and gets every generic check for
# free, without that file being rewritten.


@dataclass(frozen=True)
class Expectation:
    """One RFC 6901 equality check against the decoded live payload.

    ``live_versions`` of ``None`` means "every accepted version"; otherwise the
    check applies only when the live envelope carries one of those versions.
    """

    pointer: str
    equals: Any
    live_versions: tuple[int, ...] | None = None

    def applies_to(self, version: int) -> bool:
        return self.live_versions is None or version in self.live_versions


@dataclass(frozen=True)
class GateFixture:
    """Everything the gate needs to know about one older release's state."""

    source_app: str
    source_version: str
    snapshot_schema: str
    snapshot_version: int
    accepted_live_versions: tuple[int, ...]
    expectations: tuple[Expectation, ...]
    state: dict[str, Any]


# --------------------------------------------------------------------------
# Safe file access
# --------------------------------------------------------------------------


def _is_int(value: Any) -> bool:
    """True for a JSON integer.

    Python's ``bool`` is a subclass of ``int``, so ``isinstance(True, int)`` is
    ``True``. In JSON, ``true`` is not a number, and a byte array containing
    ``true`` is malformed -- so every integer check in this module goes through
    here rather than through a bare ``isinstance``.
    """

    return isinstance(value, int) and not isinstance(value, bool)


def read_regular_file(path: Path, limit: int, what: str) -> bytes:
    """Read ``path`` only if it is a regular, non-symlinked file under ``limit``.

    Two layers, because platforms differ:

    1. ``lstat`` first. It describes the path itself, not what a link points
       at, so a symlink is refused on every platform, including Windows where
       ``O_NOFOLLOW`` does not exist.
    2. ``O_NOFOLLOW`` where available, then ``fstat`` on the *open descriptor*.
       This closes the window between step 1 and the open: if the path is
       swapped for a link in between, POSIX refuses the open, and ``fstat``
       re-checks the file actually opened rather than the name.

    The read is capped at ``limit + 1`` bytes so an oversized file is detected
    without ever being loaded whole.
    """

    try:
        info = os.lstat(path)
    except FileNotFoundError:
        raise GateError(f"{what} does not exist: {path}") from None
    if stat.S_ISLNK(info.st_mode):
        raise GateError(f"{what} is a symlink, which the gate refuses: {path}")
    if not stat.S_ISREG(info.st_mode):
        raise GateError(f"{what} is not a regular file: {path}")

    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_BINARY", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise GateError(f"{what} could not be opened safely: {path}: {error}") from None
    with os.fdopen(descriptor, "rb") as handle:
        if not stat.S_ISREG(os.fstat(handle.fileno()).st_mode):
            raise GateError(f"{what} is not a regular file: {path}")
        data = handle.read(limit + 1)
    if len(data) > limit:
        raise GateError(f"{what} exceeds the {limit}-byte limit: {path}")
    return data


def _path_is_occupied(path: Path) -> bool:
    """True if anything at all is at ``path``, including a dangling symlink.

    ``Path.exists`` follows links, so a ``.corrupt`` that is a broken symlink
    would read as absent. ``lexists`` asks about the name itself.
    """

    return os.path.lexists(path)


def _decode_json(data: bytes, what: str) -> Any:
    try:
        return json.loads(data.decode("utf-8"))
    except UnicodeDecodeError as error:
        raise GateError(f"{what} is not UTF-8: {error}") from None
    except json.JSONDecodeError as error:
        raise GateError(f"{what} is not JSON: {error}") from None


# --------------------------------------------------------------------------
# Fixture loading and validation
# --------------------------------------------------------------------------


def _version_list(value: Any, what: str) -> tuple[int, ...]:
    """A non-empty list of unique non-negative integers, or an error."""

    if not isinstance(value, list) or not value:
        raise GateError(f"{what} must be a non-empty list of integers")
    if any(not _is_int(item) or item < 0 for item in value):
        raise GateError(f"{what} must contain only non-negative integers")
    if len(set(value)) != len(value):
        raise GateError(f"{what} must not repeat a version")
    return tuple(value)


def _check_keys(
    record: dict[str, Any],
    required: frozenset[str],
    optional: frozenset[str],
    what: str,
) -> None:
    """Reject missing and unknown keys alike.

    Unknown keys matter as much as missing ones: a fixture that spells
    ``liveVersion`` instead of ``liveVersions`` would otherwise silently widen
    an expectation to every version.
    """

    missing = sorted(required - record.keys())
    if missing:
        raise GateError(f"{what} is missing {', '.join(missing)}")
    unknown = sorted(record.keys() - required - optional)
    if unknown:
        raise GateError(f"{what} has unknown key(s) {', '.join(unknown)}")


def parse_fixture(document: Any) -> GateFixture:
    """Validate a decoded generic fixture and return its in-memory form."""

    if not isinstance(document, dict):
        raise GateError("upgrade fixture must be a JSON object")
    # Check the schema before the key set, so a legacy or foreign fixture is
    # reported as "wrong schema" rather than as a list of missing keys.
    if document.get("fixtureSchema") != FIXTURE_SCHEMA:
        raise GateError(
            f"upgrade fixture schema must be {FIXTURE_SCHEMA!r}, "
            f"got {document.get('fixtureSchema')!r}"
        )
    _check_keys(
        document, FIXTURE_KEYS_REQUIRED, FIXTURE_KEYS_OPTIONAL, "upgrade fixture"
    )
    if (
        not _is_int(document["fixtureVersion"])
        or document["fixtureVersion"] != FIXTURE_VERSION
    ):
        raise GateError("upgrade fixture has an unsupported fixtureVersion")

    source_app = document["sourceApp"]
    if not isinstance(source_app, str) or APP_ID.fullmatch(source_app) is None:
        raise GateError("upgrade fixture sourceApp must match ^[a-z0-9][a-z0-9-]*$")
    source_version = document["sourceVersion"]
    if not isinstance(source_version, str) or SEMVER.fullmatch(source_version) is None:
        raise GateError(
            f"{source_app}: upgrade fixture sourceVersion must be strict SemVer"
        )
    if "provenance" in document and not isinstance(document["provenance"], str):
        raise GateError(f"{source_app}: upgrade fixture provenance must be a string")

    snapshot_schema = document["snapshotSchema"]
    if not isinstance(snapshot_schema, str) or not snapshot_schema:
        raise GateError(
            f"{source_app}: upgrade fixture snapshotSchema must be a non-empty string"
        )
    snapshot_version = document["snapshotVersion"]
    if not _is_int(snapshot_version) or snapshot_version < 0:
        raise GateError(
            f"{source_app}: upgrade fixture snapshotVersion must be a non-negative integer"
        )
    accepted = _version_list(
        document["acceptedLiveVersions"], f"{source_app}: acceptedLiveVersions"
    )
    if not isinstance(document["state"], dict):
        raise GateError(f"{source_app}: upgrade fixture state must be a JSON object")

    raw_expectations = document["expectations"]
    if not isinstance(raw_expectations, list) or not raw_expectations:
        raise GateError(
            f"{source_app}: upgrade fixture expectations must be a non-empty list"
        )
    expectations: list[Expectation] = []
    for index, raw in enumerate(raw_expectations):
        what = f"{source_app}: expectations[{index}]"
        if not isinstance(raw, dict):
            raise GateError(f"{what} must be an object")
        _check_keys(raw, EXPECTATION_KEYS_REQUIRED, EXPECTATION_KEYS_OPTIONAL, what)
        pointer = raw["pointer"]
        if not isinstance(pointer, str):
            raise GateError(f"{what}.pointer must be a string")
        # Parse now, so a malformed pointer fails at load time on every runner
        # rather than only on the version it is scoped to.
        parse_pointer(pointer)
        live_versions = None
        if "liveVersions" in raw:
            live_versions = _version_list(raw["liveVersions"], f"{what}.liveVersions")
            outside = sorted(set(live_versions) - set(accepted))
            if outside:
                raise GateError(
                    f"{what}.liveVersions names version(s) {outside} "
                    "that acceptedLiveVersions does not accept"
                )
        expectations.append(Expectation(pointer, raw["equals"], live_versions))

    # No vacuous pass: every version the gate accepts must be checked by at
    # least one expectation. Otherwise adding a version to the accepted list
    # without describing where its data lives would make the gate check nothing
    # for precisely the case it was just extended to cover.
    unchecked = [
        version
        for version in accepted
        if not any(expectation.applies_to(version) for expectation in expectations)
    ]
    if unchecked:
        raise GateError(
            f"{source_app}: accepted live version(s) {unchecked} have no applicable expectation"
        )

    return GateFixture(
        source_app=source_app,
        source_version=source_version,
        snapshot_schema=snapshot_schema,
        snapshot_version=snapshot_version,
        accepted_live_versions=accepted,
        expectations=tuple(expectations),
        state=document["state"],
    )


def load_fixture(path: Path) -> GateFixture:
    """Read and validate a generic fixture file."""

    data = read_regular_file(path, MAX_FIXTURE_BYTES, "upgrade fixture")
    return parse_fixture(_decode_json(data, f"upgrade fixture {path}"))


# --------------------------------------------------------------------------
# JSON pointers (RFC 6901)
# --------------------------------------------------------------------------


def parse_pointer(pointer: str) -> list[str]:
    """Split an RFC 6901 pointer into unescaped reference tokens.

    ===============  ==========================
    pointer          tokens
    ===============  ==========================
    ``""``           ``[]`` (the whole document)
    ``"/a/0"``       ``["a", "0"]``
    ``"/a~1b"``      ``["a/b"]``
    ``"/m~0n"``      ``["m~n"]``
    ``"/"``          ``[""]`` (the key ``""``)
    ===============  ==========================

    Order matters when unescaping: ``~1`` must become ``/`` *before* ``~0``
    becomes ``~``. Doing it the other way turns ``~01`` -- the literal key
    ``~1`` -- into ``~1`` and then into ``/``. A ``~`` followed by anything
    other than ``0`` or ``1`` is malformed and rejected.
    """

    if pointer == "":
        return []
    if not pointer.startswith("/"):
        raise GateError(f"JSON pointer must be empty or start with '/': {pointer!r}")
    tokens = pointer[1:].split("/")
    for token in tokens:
        if re.search(r"~(?![01])", token):
            raise GateError(f"JSON pointer has an invalid '~' escape: {pointer!r}")
    return [token.replace("~1", "/").replace("~0", "~") for token in tokens]


_MISSING = object()


def resolve_pointer(document: Any, pointer: str) -> Any:
    """Return the value ``pointer`` names, or the ``_MISSING`` sentinel.

    A sentinel rather than ``None`` because ``null`` is a perfectly good JSON
    value to expect; "absent" and "present and null" must stay distinct.
    """

    current = document
    for token in parse_pointer(pointer):
        if isinstance(current, dict):
            if token not in current:
                return _MISSING
            current = current[token]
        elif isinstance(current, list):
            if ARRAY_INDEX.fullmatch(token) is None:
                return _MISSING
            index = int(token)
            if index >= len(current):
                return _MISSING
            current = current[index]
        else:
            # A scalar has no children; a pointer through one names nothing.
            return _MISSING
    return current


def json_equal(left: Any, right: Any) -> bool:
    """JSON equality, which is stricter than Python's ``==``.

    ================  ============  ===========
    left, right       Python ``==``  JSON
    ================  ============  ===========
    ``True``, ``1``   equal          different
    ``1``, ``1.0``    equal          equal
    ``1``, ``"1"``    different      different
    ================  ============  ===========

    Booleans compare only with booleans; numbers compare by value whether
    integral or not (JSON has one number type); containers compare element by
    element with the same rule. NaN never equals anything, but strict JSON
    cannot contain one anyway.
    """

    if isinstance(left, bool) or isinstance(right, bool):
        return isinstance(left, bool) and isinstance(right, bool) and left is right
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        if isinstance(left, float) and math.isnan(left):
            return False
        return left == right
    if isinstance(left, dict) and isinstance(right, dict):
        return left.keys() == right.keys() and all(
            json_equal(left[key], right[key]) for key in left
        )
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(
            json_equal(a, b) for a, b in zip(left, right)
        )
    if left is None or right is None:
        return left is None and right is None
    if isinstance(left, str) and isinstance(right, str):
        return left == right
    return False


# --------------------------------------------------------------------------
# materialize
# --------------------------------------------------------------------------


def encode_state(state: dict[str, Any]) -> bytes:
    """The payload bytes an adapter would have written for ``state``.

    Compact separators, sorted keys, UTF-8 with non-ASCII left as is: this is
    serde_json's output for a ``serde_json::Value``, whose maps are sorted.
    For Engram v0.4.0 it reproduces the release's own bytes exactly.
    """

    return json.dumps(
        state,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def encode_envelope(schema: str, version: int, payload: bytes) -> bytes:
    """The host's persistence envelope, compact and sorted, plus ``\\n``.

    Returned as bytes so the newline is ``\\n`` on every platform; writing it
    in text mode would make it ``\\r\\n`` on Windows.
    """

    envelope = {"schema": schema, "version": version, "bytes": list(payload)}
    return (json.dumps(envelope, separators=(",", ":"), sort_keys=True) + "\n").encode(
        "utf-8"
    )


def materialize_fixture(fixture: GateFixture, output: Path) -> Path:
    """Write ``fixture``'s state at ``output`` exactly as its release did."""

    refusal = (
        f"{fixture.source_app}: refusing to write state through a "
        f"non-regular path: {output}"
    )
    if _path_is_occupied(output):
        info = os.lstat(output)
        if stat.S_ISLNK(info.st_mode) or not stat.S_ISREG(info.st_mode):
            raise GateError(refusal)
    output.parent.mkdir(parents=True, exist_ok=True)
    envelope = encode_envelope(
        fixture.snapshot_schema,
        fixture.snapshot_version,
        encode_state(fixture.state),
    )
    # The same two layers as ``read_regular_file``: the ``lstat`` above works
    # everywhere, and ``O_NOFOLLOW`` closes the race on platforms that have it.
    # 0o600 because this is a stand-in for one person's private data.
    flags = (
        os.O_WRONLY
        | os.O_CREAT
        | os.O_TRUNC
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_BINARY", 0)
    )
    try:
        descriptor = os.open(output, flags, 0o600)
    except OSError as error:
        raise GateError(f"{refusal} ({error})") from None
    with os.fdopen(descriptor, "wb") as handle:
        handle.write(envelope)
    return output


# --------------------------------------------------------------------------
# verify
# --------------------------------------------------------------------------


def corrupt_sibling(state_path: Path) -> Path:
    """Where every generated host moves a state file it rejected."""

    return Path(f"{state_path}.corrupt")


def verify_state(fixture: GateFixture, state_path: Path) -> dict[str, Any]:
    """Prove the new build kept ``fixture``'s state. Returns the decoded payload.

    The checks run in the order a person debugging a failure would want them:
    first "is there a file", then "did the host give up on it", then the
    envelope, then the data. The first failure is the one reported.

    Returning the payload lets an app with checks the pointer language cannot
    express -- TaskApp's "exactly one task with this name" -- run them on data
    the generic half has already validated.
    """

    app = fixture.source_app
    # Did the host reject a file it should have accepted? This is checked
    # first because it is the most specific diagnosis available. A host that
    # quarantined the fixture either wrote a fresh empty state -- which would
    # otherwise fail later with a confusing "pointer missing" -- or wrote
    # nothing, which would otherwise read as "the state does not exist".
    corrupt = corrupt_sibling(state_path)
    if _path_is_occupied(corrupt):
        raise GateError(f"{app}: quarantined a compatible upgrade fixture: {corrupt}")

    # Is it there at all? (The size cap and symlink refusal live inside.)
    data = read_regular_file(state_path, MAX_STATE_BYTES, f"{app}: upgrade state")

    snapshot = _decode_json(data, f"{app}: live state")
    if not isinstance(snapshot, dict):
        raise GateError(f"{app}: live state must be a JSON object")
    if snapshot.get("schema") != fixture.snapshot_schema:
        raise GateError(
            f"{app}: live state changed the snapshot schema "
            f"(expected {fixture.snapshot_schema!r}, got {snapshot.get('schema')!r})"
        )
    version = snapshot.get("version")
    if not _is_int(version) or version not in fixture.accepted_live_versions:
        raise GateError(
            f"{app}: live state changed the snapshot version to {version!r}; "
            f"accepted versions are {list(fixture.accepted_live_versions)}"
        )

    encoded = snapshot.get("bytes")
    if not isinstance(encoded, list) or any(
        not _is_int(value) or not 0 <= value <= 255 for value in encoded
    ):
        raise GateError(f"{app}: live state has invalid snapshot bytes")
    payload = _decode_json(bytes(encoded), f"{app}: snapshot bytes")
    if not isinstance(payload, dict):
        raise GateError(f"{app}: snapshot bytes must decode to a JSON object")

    for expectation in fixture.expectations:
        if not expectation.applies_to(version):
            continue
        actual = resolve_pointer(payload, expectation.pointer)
        if actual is _MISSING:
            raise GateError(
                f"{app}: live v{version} payload has nothing at {expectation.pointer!r}"
            )
        if not json_equal(actual, expectation.equals):
            raise GateError(
                f"{app}: live v{version} payload at {expectation.pointer!r} is "
                f"{actual!r}, expected {expectation.equals!r}"
            )
    return payload


# --------------------------------------------------------------------------
# verify-quarantine
# --------------------------------------------------------------------------


def verify_quarantine(state_path: Path, probe_path: Path, app: str = "app") -> None:
    """Prove the host moved rejected bytes to ``.corrupt`` without changing them.

    The release lane writes garbage to the live path, copies it to the probe,
    and relaunches. Three outcomes are wrong, and each gets its own message:

    - no ``.corrupt`` -- the host deleted or "repaired" the user's data;
    - ``.corrupt`` differs from the probe -- it rewrote the bytes first;
    - the live path still equals the probe -- it copied instead of moving, so
      every later launch quarantines again and the app never recovers.
    """

    probe = read_regular_file(probe_path, MAX_STATE_BYTES, f"{app}: quarantine probe")
    corrupt_path = corrupt_sibling(state_path)
    quarantined = read_regular_file(
        corrupt_path, MAX_STATE_BYTES, f"{app}: quarantined state"
    )
    if quarantined != probe:
        raise GateError(
            f"{app}: quarantined state differs from the probe: {corrupt_path}"
        )
    if _path_is_occupied(state_path):
        live = read_regular_file(state_path, MAX_STATE_BYTES, f"{app}: live state")
        if live == probe:
            raise GateError(
                f"{app}: rejected state was copied, not moved, out of {state_path}"
            )


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    subparsers = parser.add_subparsers(dest="command", required=True)

    materialize = subparsers.add_parser(
        "materialize", help="write a fixture's state in the host envelope"
    )
    materialize.add_argument("--fixture", type=Path, required=True)
    materialize.add_argument("--output", type=Path, required=True)

    verify = subparsers.add_parser(
        "verify", help="prove a new build kept a fixture's state"
    )
    verify.add_argument("--fixture", type=Path, required=True)
    verify.add_argument("--state", type=Path, required=True)

    quarantine = subparsers.add_parser(
        "verify-quarantine", help="prove rejected state was moved aside unchanged"
    )
    quarantine.add_argument("--state", type=Path, required=True)
    quarantine.add_argument("--probe", type=Path, required=True)
    quarantine.add_argument(
        "--app", default="app", help="product name for error messages"
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "materialize":
            materialize_fixture(load_fixture(args.fixture), args.output)
        elif args.command == "verify":
            verify_state(load_fixture(args.fixture), args.state)
        else:
            verify_quarantine(args.state, args.probe, args.app)
    except (GateError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
