"""Keep the repository-source-input boundary, its projections and its pins in sync.

The problem this solves
=======================

``code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json``
lists, for a few boundaries, *every package that has some property* (every Rust
crate that loads ``rust_library.star``, every TypeScript workspace root, ...).
Those lists are recomputed from the repository by
``build_tool_conformance.expected_repository_source_exact_roots`` and compared
against the fixture by the corpus test.  Every new package therefore changed:

1. the fixture itself,
2. the C# projection   (``SourceInputRegistries.Generated.cs``),
3. the Swift projection (``RepositorySourceInputBoundary.swift``),
4. the boundary digest, which is pinned in 27 files across five languages,
5. ``scope_count`` / ``authorization_count`` in the Python corpus test.

Doing that by hand was error-prone (it needed four follow-up PRs) and, worse,
the corpus test only runs on PRs the CI gate thinks are relevant, so the PR that
*added* the package merged green and broke main.

The model
=========

Think of the boundary as a tiny build system with one input and many outputs::

    repository tree --(discovery)--> exact_roots --(json)--> fixture
                                                      |
                          +---------------------------+------------------+
                          v                           v                  v
                  C# / Swift projection         SHA-256 digest      scope counts
                                                      |
                                  6 digest pins in 5 languages + case fixtures

``plan()`` computes *every* output in memory and compares it with what is on
disk.  ``sync-boundary`` writes the differences; ``sync-boundary --check``
only reports them.  Because both use the same plan, "check passes" is exactly
"sync would change nothing".

Failing closed
==============

Each place we rewrite is located by an *exact, anchored* pattern that must
match exactly the expected number of times.  If a pattern finds nothing, or
finds something twice, or a file has moved, the tool stops with an error
instead of guessing.  It never does a blind search-and-replace of a digest
across the repository.  The one repository-wide safety net is a *bounded scan*
for the old digest in the build-tool lanes, which refuses to continue if the
digest is pinned somewhere this module does not know about.
"""

from __future__ import annotations

import argparse
import copy
import difflib
import json
import os
import re
import sys
import tempfile
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path

import build_tool_conformance as runner

#: The command a human runs to repair drift.  Quoted in every failure message.
SYNC_COMMAND = "python3 code/scripts/build_tool_conformance.py sync-boundary"

FIXTURE_DIR = Path("code/specs/fixtures/build-tool-v1")
FIXTURE_PATH = FIXTURE_DIR / "repository-source-input-boundary.json"
SCHEMA_PATH = FIXTURE_DIR / "repository-source-input-boundary.schema.json"
REGISTRY_PATH = FIXTURE_DIR / "language-source-input-registry.json"
CASES_DIR = FIXTURE_DIR / "cases"
PYTHON_TEST_PATH = Path("code/scripts/tests/test_build_tool_conformance_runner.py")

HEX64 = r"[0-9a-f]{64}"


class SyncError(Exception):
    """The boundary cannot be synchronised safely; a human must look."""


# --------------------------------------------------------------------------
# Where the digest is pinned.  Each entry is (path, anchored regex).  The regex
# has exactly three groups: text before the digest, the digest, text after.
# --------------------------------------------------------------------------
DIGEST_PINS: tuple[tuple[Path, str], ...] = (
    (
        Path("code/programs/go/build-tool/internal/graphdiff/graphdiff_test.go"),
        rf'(\tif digest != ")({HEX64})(" \{{\n\t\tt\.Fatalf\("boundary digest = %s", digest\))',
    ),
    (
        Path("code/programs/dotnet/build-tool-csharp/SourceHashing.cs"),
        rf'(public const string RepositorySourceInputBoundaryDigest =\n\s+")({HEX64})(";)',
    ),
    (
        Path(
            "code/programs/dotnet/build-tool-csharp/tests/BuildTool.CSharp.Tests/"
            "HasherConformanceTests.cs"
        ),
        rf'(Assert\.Equal\(\n\s+")({HEX64})(",\n\s+Hasher\.RepositorySourceInputBoundaryDigest\);)',
    ),
    (
        Path("code/programs/swift/build-tool/Sources/BuildToolCore/Hasher.swift"),
        rf'(static let repositorySourceInputBoundaryDigest =\n\s+")({HEX64})(")',
    ),
    (
        Path(
            "code/programs/swift/build-tool/Tests/BuildToolCoreTests/HasherTests.swift"
        ),
        rf'(Hasher\.repositorySourceInputBoundaryDigest\n\s+== ")({HEX64})(")',
    ),
    (
        PYTHON_TEST_PATH,
        rf'(runner\.repository_source_input_boundary_digest\(boundary\),\n\s+")({HEX64})(",)',
    ),
)

#: Every case fixture that pins the digest does so with this exact key.
CASE_PIN = rf'("boundary_sha256": ")({HEX64})(")'

#: The summary dict the Python test pins.  Group 1 is everything up to the
#: first number; groups 2-5 are the text between numbers and after the last.
COUNT_PIN = (
    r'(self\.assertEqual\(\n\s+summary,\n\s+\{\n\s+"boundary_count": )\d+'
    r'(,\n\s+"input_count": )\d+'
    r'(,\n\s+"scope_count": )\d+'
    r'(,\n\s+"authorization_count": )\d+'
    r"(,\n)"
)

#: The directories scanned for *unregistered* copies of a stale digest.  Every
#: non-binary file below them is read; there is no suffix allowlist, so a pin
#: in a language this module has never heard of is still found.
SCAN_ROOTS = (
    "code/programs",
    "code/scripts",
    "code/specs/fixtures/build-tool-v1",
    ".github",
)

#: Generated or vendored directories that are never scanned.
SCAN_SKIP_DIRS = frozenset(
    {".git", "node_modules", "target", ".build", "bin", "obj", ".gradle", "__pycache__"}
)

#: Dated narrative that deliberately keeps the digest an earlier PR validated.
#: These record history, not a live pin, and must never be rewritten.
NARRATIVE_EXCLUSIONS = frozenset(
    {
        Path("code/specs/package-parity-roadmap.md"),
        Path(".claude/package-parity-loop-state.json"),
    }
)


# --------------------------------------------------------------------------
# Projections: the JSON document embedded verbatim in C# and Swift sources.
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class Projection:
    """A source file embedding the fixture text between two exact markers."""

    path: Path
    start: str  # text that ends the line opening the literal
    end: str  # text that begins the line closing the literal
    indent: str  # prefix added to every non-empty embedded line


PROJECTIONS = (
    Projection(
        Path(
            "code/programs/dotnet/build-tool-csharp/SourceInputRegistries.Generated.cs"
        ),
        start='internal const string RepositoryBoundaryJson = """\n',
        end='\n""";',
        indent="",
    ),
    Projection(
        Path(
            "code/programs/swift/build-tool/Sources/BuildToolCore/"
            "RepositorySourceInputBoundary.swift"
        ),
        start='private static let embeddedJSON = #"""\n',
        end='\n        """#',
        indent="        ",
    ),
)


def render_projection(projection: Projection, fixture_text: str, current: str) -> str:
    """Return ``current`` with the embedded JSON replaced by ``fixture_text``."""

    # The opener must be unambiguous.  The closer need not be unique (the C#
    # file closes an earlier, unrelated literal with the same text), so take
    # the first one *after* the opener and insist that one exists.
    if current.count(projection.start) != 1:
        raise SyncError(
            f"{projection.path}: expected exactly one "
            f"{projection.start.strip()!r} opener; the projection format is "
            "not recognised"
        )
    begin = current.index(projection.start) + len(projection.start)
    finish = current.find(projection.end, begin)
    if finish < 0:
        raise SyncError(
            f"{projection.path}: no {projection.end.strip()!r} closer after the "
            "opener; the projection format is not recognised"
        )
    body = "\n".join(
        projection.indent + line if line else line
        for line in fixture_text.rstrip("\n").split("\n")
    )
    return current[:begin] + body + current[finish:]


# --------------------------------------------------------------------------
# Small helpers for exact, counted replacement.
# --------------------------------------------------------------------------


def replace_exactly_once(path: Path, text: str, pattern: str, replacement: str) -> str:
    """Substitute ``replacement`` for the single match of ``pattern``."""

    matches = list(re.finditer(pattern, text))
    if len(matches) != 1:
        raise SyncError(
            f"{path}: expected exactly one match of the pinned-format pattern "
            f"{pattern!r}, found {len(matches)}; the file format is not recognised"
        )
    match = matches[0]
    return text[: match.start()] + match.expand(replacement) + text[match.end() :]


def pin_digest(path: Path, text: str, pattern: str, digest: str) -> tuple[str, str]:
    """Rewrite the digest in ``text``; return ``(new_text, digest_found)``."""

    matches = list(re.finditer(pattern, text))
    if len(matches) != 1:
        raise SyncError(
            f"{path}: expected exactly one boundary-digest pin matching "
            f"{pattern!r}, found {len(matches)}; the pin format is not recognised"
        )
    match = matches[0]
    new = text[: match.start(2)] + digest + text[match.end(2) :]
    return new, match.group(2)


# --------------------------------------------------------------------------
# The plan.
# --------------------------------------------------------------------------


@dataclass
class SyncPlan:
    """Everything ``sync-boundary`` would change, computed without writing."""

    repo_root: Path
    old_digest: str
    new_digest: str
    summary: dict[str, int]
    #: path -> (current text, desired text); only entries that differ.
    changes: dict[Path, tuple[str, str]] = field(default_factory=dict)
    #: boundary id -> (roots to add, roots to drop)
    root_changes: dict[str, tuple[list[str], list[str]]] = field(default_factory=dict)

    @property
    def in_sync(self) -> bool:
        return not self.changes

    def describe(self) -> str:
        """Human-readable account of exactly what is out of sync."""

        lines = []
        for boundary_id, (added, dropped) in sorted(self.root_changes.items()):
            for root in added:
                lines.append(f"  + {boundary_id}: {root}")
            for root in dropped:
                lines.append(f"  - {boundary_id}: {root}")
        if self.old_digest != self.new_digest:
            lines.append(f"  boundary digest: {self.old_digest}")
            lines.append(f"              ->   {self.new_digest}")
        lines.append(f"  files that would change ({len(self.changes)}):")
        for path in sorted(self.changes, key=str):
            lines.append(f"    {path}")
        return "\n".join(lines)

    def unified_diff(self, context: int = 0, limit: int = 40) -> str:
        """Unified diffs, each truncated to ``limit`` lines.

        The 21 case fixtures differ only by the digest line, which the digest
        summary already shows, so they are listed but not diffed.
        """

        chunks = []
        for path in sorted(self.changes, key=str):
            if CASES_DIR in path.parents:
                continue
            before, after = self.changes[path]
            diff = list(
                difflib.unified_diff(
                    before.splitlines(),
                    after.splitlines(),
                    f"a/{path}",
                    f"b/{path}",
                    lineterm="",
                    n=context,
                )
            )
            if len(diff) > limit:
                diff = [*diff[:limit], f"... ({len(diff) - limit} more diff lines)"]
            chunks.append("\n".join(diff))
        return "\n".join(chunks)


def _checked_path(repo_root: Path, path: Path) -> Path:
    """Return ``repo_root / path`` after refusing symlinks and escapes.

    A target that is a symlink could redirect a write (or a read) to a file the
    tool was never meant to touch, and a path that resolves outside the
    repository is never legitimate, so both are errors rather than followed.
    """

    full = repo_root / path
    if full.is_symlink():
        raise SyncError(f"{path}: is a symlink; refusing to read or write through it")
    try:
        full.resolve().relative_to(repo_root.resolve())
    except ValueError as error:
        raise SyncError(f"{path}: resolves outside the repository") from error
    return full


def _read(repo_root: Path, path: Path) -> str:
    full = _checked_path(repo_root, path)
    try:
        return full.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise SyncError(
            f"{path}: cannot read a file the boundary sync needs ({error})"
        ) from error


def _dump(document: object) -> str:
    """Canonical text of the fixture: 2-space indent, UTF-8, trailing newline."""

    return json.dumps(document, indent=2, ensure_ascii=False) + "\n"


def _scan_for_unregistered_pins(
    repo_root: Path, digests: set[str], registered: set[Path]
) -> list[Path]:
    """Find files outside ``registered`` that still hold a stale digest.

    Registered targets are excluded here only because ``plan`` checks their
    *desired* content directly.  Every other non-binary file under
    ``SCAN_ROOTS`` is read; a file that cannot be read is an error, never a
    silent skip.
    """

    needles = [digest.encode("ascii") for digest in sorted(digests)]
    found: list[Path] = []
    for scan_root in SCAN_ROOTS:

        def walk_error(error: OSError) -> None:
            raise SyncError(f"cannot scan a directory for stale pins: {error}")

        for directory, subdirectories, filenames in os.walk(
            repo_root / scan_root, onerror=walk_error
        ):
            subdirectories[:] = [d for d in subdirectories if d not in SCAN_SKIP_DIRS]
            for name in filenames:
                full = Path(directory) / name
                relative = full.relative_to(repo_root)
                if (
                    relative in registered
                    or relative in NARRATIVE_EXCLUSIONS
                    or full.is_symlink()
                ):
                    continue
                try:
                    data = full.read_bytes()
                except OSError as error:
                    raise SyncError(
                        f"{relative}: cannot be read while scanning for stale "
                        f"digest pins ({error})"
                    ) from error
                if b"\0" in data:  # binary: not a text pin site
                    continue
                if any(needle in data for needle in needles):
                    found.append(relative)
    return sorted(found, key=str)


def plan(
    repo_root: Path = runner.REPO_ROOT,
    *,
    discover: Callable[[Path], dict[str, list[str]]] | None = None,
) -> SyncPlan:
    """Compute the full synchronised state and diff it against the disk.

    ``discover`` exists so unit tests can supply roots without building a
    repository; production always uses the function the corpus test uses.
    """

    discover = discover or runner.expected_repository_source_exact_roots

    # 1. Current fixture, and proof that we understand its on-disk format.
    fixture_text = _read(repo_root, FIXTURE_PATH)
    try:
        current = json.loads(fixture_text)
    except json.JSONDecodeError as error:
        raise SyncError(f"{FIXTURE_PATH}: not valid JSON ({error})") from error
    if _dump(current) != fixture_text:
        raise SyncError(
            f"{FIXTURE_PATH}: not in canonical form (2-space indent, UTF-8, "
            "trailing newline); refusing to rewrite a file in an unknown format"
        )

    # 2. Desired roots, from the shared discovery function.
    derived = discover(repo_root)
    boundaries = {entry["id"]: entry for entry in current["boundaries"]}
    desired = copy.deepcopy(current)
    desired_by_id = {entry["id"]: entry for entry in desired["boundaries"]}
    root_changes: dict[str, tuple[list[str], list[str]]] = {}
    for boundary_id, roots in derived.items():
        if boundary_id not in boundaries:
            raise SyncError(
                f"discovery produced boundary {boundary_id!r} which is not in "
                f"{FIXTURE_PATH}; add the boundary by hand first"
            )
        have = boundaries[boundary_id]["applies_to"]["exact_roots"]
        if have != roots:
            root_changes[boundary_id] = (
                [r for r in roots if r not in have],
                [r for r in have if r not in roots],
            )
        desired_by_id[boundary_id]["applies_to"]["exact_roots"] = list(roots)

    # 3. Validate the desired document with the production validator.  This is
    #    what rejects an emptied boundary, unsorted roots or case collisions.
    schema = runner.load_document(repo_root / SCHEMA_PATH)
    registry = runner.load_document(repo_root / REGISTRY_PATH)
    try:
        summary = runner._validate_repository_source_input_boundary(
            desired, schema, registry
        )
    except runner.ConformanceError as error:
        raise SyncError(
            f"the synchronised boundary is invalid ({error.code}: {error.message}); "
            "fix the boundary by hand"
        ) from error
    new_digest = runner.repository_source_input_boundary_digest(desired)
    desired_text = _dump(desired)

    targets: dict[Path, str] = {FIXTURE_PATH: desired_text}
    old_values: set[str] = set()

    def current_text(path: Path) -> str:
        return targets[path] if path in targets else _read(repo_root, path)

    # 4. Projections.
    for projection in PROJECTIONS:
        targets[projection.path] = render_projection(
            projection, desired_text, _read(repo_root, projection.path)
        )

    # 5. Digest pins in source files.
    for path, pattern in DIGEST_PINS:
        text, found = pin_digest(path, current_text(path), pattern, new_digest)
        old_values.add(found)
        targets[path] = text

    # 6. Digest pins in case fixtures: every file that has the key, no other.
    case_names = sorted(
        name for name in os.listdir(repo_root / CASES_DIR) if name.endswith(".json")
    )
    pinned_cases = 0
    for name in case_names:
        path = CASES_DIR / name
        text = _read(repo_root, path)
        occurrences = text.count('"boundary_sha256"')
        if occurrences == 0:
            continue
        matches = re.findall(CASE_PIN, text)
        if occurrences != len(matches):
            raise SyncError(
                f"{path}: a boundary_sha256 key does not hold a 64-hex digest "
                "in the recognised format"
            )
        old_values.update(match[1] for match in matches)
        targets[path] = re.sub(CASE_PIN, rf"\g<1>{new_digest}\g<3>", text)
        pinned_cases += 1
    if pinned_cases == 0:
        raise SyncError(
            f"{CASES_DIR}: no case fixture pins boundary_sha256; unexpected"
        )

    # 7. Counts the Python test pins: one four-line dict, anchored on the
    #    `self.assertEqual(summary, {...})` that precedes the digest assertion.
    test_text = replace_exactly_once(
        PYTHON_TEST_PATH,
        current_text(PYTHON_TEST_PATH),
        COUNT_PIN,
        r"\g<1>{boundary_count}\g<2>{input_count}\g<3>{scope_count}"
        r"\g<4>{authorization_count}\g<5>".format(**summary),
    )
    targets[PYTHON_TEST_PATH] = test_text

    # 8. Safety net.  A digest that is being replaced must not survive
    #    anywhere: not in a second place inside a file we rewrite (another key
    #    of a case fixture, a duplicated constant), and not in a file this
    #    module does not know about.
    old_digest = runner.repository_source_input_boundary_digest(current)
    stale = (old_values | {old_digest}) - {new_digest}
    for path in sorted(targets, key=str):
        for digest in sorted(stale):
            if digest in targets[path]:
                raise SyncError(
                    f"{path}: still contains the stale boundary digest {digest} "
                    "after substitution (a second copy in an unrecognised place); "
                    "refusing to write a half-updated tree"
                )
    # Nothing is stale when the tree is already in sync; skip the wide scan.
    unknown = (
        _scan_for_unregistered_pins(repo_root, stale, set(targets)) if stale else []
    )
    if unknown:
        listing = "\n".join(f"  {path}" for path in unknown)
        raise SyncError(
            "the old boundary digest is pinned in files this tool does not "
            f"know how to rewrite:\n{listing}\n"
            "Teach build_tool_boundary_sync.DIGEST_PINS about them."
        )

    changes = {}
    for path, desired_content in targets.items():
        existing = _read(repo_root, path)
        if existing != desired_content:
            changes[path] = (existing, desired_content)

    if (stale or old_digest != new_digest) and not changes:
        raise SyncError(
            "the boundary changed but no file would be updated; refusing to "
            "report success"
        )
    return SyncPlan(
        repo_root=repo_root,
        old_digest=old_digest,
        new_digest=new_digest,
        summary=summary,
        changes=changes,
        root_changes=root_changes,
    )


def _atomic_write(path: Path, text: str) -> None:
    """Replace ``path`` with ``text`` atomically, keeping its permissions.

    The text goes to a temporary file in the same directory and is moved over
    the target with ``os.replace``.  A reader never sees a half-written file,
    and a link at ``path`` would be replaced rather than written through.
    """

    mode = path.stat().st_mode & 0o7777
    descriptor, temporary = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as handle:
            handle.write(text)
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    except BaseException:
        if os.path.exists(temporary):
            os.unlink(temporary)
        raise


def apply(sync_plan: SyncPlan) -> list[Path]:
    """Write every change in ``sync_plan``; return the paths written."""

    # Validate every target before writing any, so a refused path leaves the
    # tree untouched.
    targets = {
        path: _checked_path(sync_plan.repo_root, path) for path in sync_plan.changes
    }
    written = []
    for path in sorted(sync_plan.changes, key=str):
        _atomic_write(targets[path], sync_plan.changes[path][1])
        written.append(path)
    return written


def run(
    arguments: Iterable[str] | None = None,
    repo_root: Path = runner.REPO_ROOT,
    *,
    discover: Callable[[Path], dict[str, list[str]]] | None = None,
) -> int:
    """Entry point for ``build_tool_conformance.py sync-boundary [--check]``.

    Exit codes: 0 in sync (or fixed), 1 ``--check`` found drift, 2 error.
    """

    parser = argparse.ArgumentParser(prog="sync-boundary")
    parser.add_argument(
        "--check", action="store_true", help="change nothing; exit 1 if out of sync"
    )
    options = parser.parse_args(list(arguments) if arguments is not None else None)
    try:
        sync_plan = plan(repo_root, discover=discover)
    except SyncError as error:
        print(f"sync-boundary: error: {error}", file=sys.stderr)
        return 2

    if options.check:
        if sync_plan.in_sync:
            print("sync-boundary: repository-source-input boundary is in sync")
            return 0
        print(
            "sync-boundary: the repository-source-input boundary is OUT OF SYNC "
            "with the repository.\n"
            f"{sync_plan.describe()}\n\n{sync_plan.unified_diff()}\n\n"
            f"Fix it by running:\n  {SYNC_COMMAND}\nand commit the changes.",
            file=sys.stderr,
        )
        return 1

    written = apply(sync_plan)
    if not written:
        print("sync-boundary: already in sync; nothing to do")
        return 0
    print(f"sync-boundary: updated {len(written)} files")
    print(sync_plan.describe())
    return 0
