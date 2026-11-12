#!/usr/bin/env python3
"""Static guard that every Elixir project reaching a Hex dependency is locked.

Why this exists
---------------
Mix resolves ``{:jason, "~> 1.4"}`` to a concrete release and records the
answer in ``mix.lock``.  If the lock is missing, ``mix deps.get`` quietly picks
whatever is newest on Hex that day; if the lock no longer satisfies
``mix.exs``, Mix quietly *rewrites* it.  Either way CI would consume a
dependency nobody reviewed.

Mix has a ``--check-locked`` flag for this, but no environment variable
(verified against the Mix 1.19 beams: the only MIX_* variables are things like
MIX_ENV, MIX_HOME and MIX_DEPS_PATH), and 350+ BUILD files call
``mix deps.get``.  So the enforcement lives here, as a cheap static check that
needs no Elixir installation and runs in the ``contracts`` CI job.

What counts as "reaching" a Hex dependency
------------------------------------------
A project's *closure* is the project itself plus every ``path:`` dependency,
transitively.  Hex dependencies enter the closure in two ways:

* the project's own ``mix.exs`` lists them (any ``only:`` environment, since
  ``mix test`` builds the test environment of the root project), or
* a path dependency lists them *without* ``only:``.  Mix ignores a
  dependency's test-only deps when it is consumed by someone else, so
  ``{:jason, "~> 1.4", only: :test}`` in a library does not leak to its users.

A project whose closure contains at least one Hex dependency is a *root* and
must carry a tracked ``mix.lock``.

Checks (``check`` sub-command)
------------------------------
1. every root has a tracked ``mix.lock``;
2. every Hex dependency in a root's closure appears in that lock, and the
   locked version satisfies the ``mix.exs`` requirement;
3. every ``:hex`` lock entry carries both checksums (the inner one inside the
   tuple, the outer one at the end);
4. across all checked locks, each package is locked to a single version.

Usage::

    python3 code/scripts/elixir_locks.py roots
    python3 code/scripts/elixir_locks.py check [--changed-since REF]
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

# Reuse the comment stripper and the dependency-tuple regexes from the
# third-party policy so both tools agree on what a mix.exs "dependency" is.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from third_party_dependency_policy import (  # noqa: E402
    MIX_APP_DECLARATION,
    MIX_DEPENDENCY_DECLARATION,
    MIX_PATH_OPTION,
    elixir_without_comments,
)

REPO_ROOT = Path(__file__).resolve().parents[2]

# A requirement is the first quoted string of a Hex tuple: {:jason, "~> 1.4"}.
_LEADING_STRING = re.compile(r"""\s*(?P<q>['"])(?P<req>[^'"]*)(?P=q)""")
_ONLY_OPTION = re.compile(r"\bonly\s*:")
_OPTIONAL_OPTION = re.compile(r"\boptional\s*:\s*true")
_NON_HEX_SOURCE = re.compile(r"\b(?:git|github|in_umbrella)\s*:")

# One line of mix.lock for a Hex package.  Fields in order: version, inner
# checksum, build tools, deps, repo, outer checksum.
_LOCK_HEX = re.compile(
    r'^\s*"(?P<key>[^"]+)"\s*:\s*\{\s*:hex\s*,\s*:(?P<name>\w+)\s*,\s*'
    r'"(?P<version>[^"]+)"\s*,\s*"(?P<inner>[^"]*)"\s*,.*?,\s*'
    r'"(?P<repo>[^"]*)"\s*(?:,\s*"(?P<outer>[^"]*)"\s*)?\}\s*,?\s*$'
)
_SHA256 = re.compile(r"^[0-9a-f]{64}$")


# ---------------------------------------------------------------------------
# Version requirements
# ---------------------------------------------------------------------------


def parse_version(text: str) -> tuple[int, int, int]:
    """Parse ``1.4.5`` (or ``1.4``) into a comparable triple.

    Pre-release/build suffixes are dropped: the locks here pin stable releases.
    """
    core = re.split(r"[-+]", text.strip(), maxsplit=1)[0]
    parts = core.split(".")
    if not 1 <= len(parts) <= 3 or not all(p.isdigit() for p in parts):
        raise ValueError(f"unsupported version {text!r}")
    numbers = [int(p) for p in parts] + [0] * (3 - len(parts))
    return (numbers[0], numbers[1], numbers[2])


def _clause_satisfied(clause: str, version: tuple[int, int, int]) -> bool:
    match = re.fullmatch(r"(==|!=|>=|<=|~>|>|<)?\s*(\S+)", clause.strip())
    if not match:
        raise ValueError(f"unsupported requirement clause {clause!r}")
    op = match.group(1) or "=="
    bound_text = match.group(2)
    bound = parse_version(bound_text)
    if op == "==":
        return version == bound
    if op == "!=":
        return version != bound
    if op == ">=":
        return version >= bound
    if op == "<=":
        return version <= bound
    if op == ">":
        return version > bound
    if op == "<":
        return version < bound
    # Pessimistic operator.  ``~> 1.4`` allows 1.x from 1.4; ``~> 1.4.2``
    # allows 1.4.x from 1.4.2.  The number of written segments decides which
    # one is allowed to float.
    segments = len(bound_text.split("."))
    if segments >= 3:
        upper = (bound[0], bound[1] + 1, 0)
    else:
        upper = (bound[0] + 1, 0, 0)
    return bound <= version < upper


def satisfies(requirement: str, version_text: str) -> bool:
    """Return True if ``version_text`` satisfies a Mix requirement string.

    Supports ``==``, ``!=``, ``>``, ``>=``, ``<``, ``<=``, ``~>`` and the
    ``and`` / ``or`` combinations (``and`` binds tighter than ``or``).
    """
    version = parse_version(version_text)
    for alternative in re.split(r"\s+or\s+", requirement.strip()):
        clauses = re.split(r"\s+and\s+", alternative.strip())
        if all(_clause_satisfied(clause, version) for clause in clauses):
            return True
    return False


# ---------------------------------------------------------------------------
# mix.exs and mix.lock parsing
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Dep:
    name: str
    requirement: str | None  # None for path deps
    path: str | None
    test_scoped: bool  # declared with an ``only:`` option
    optional: bool


@dataclass
class Project:
    directory: Path  # absolute
    app: str
    deps: list[Dep]


def parse_mix_exs(text: str, manifest: Path) -> Project:
    """Extract the app name and dependency tuples from one ``mix.exs``."""
    source = elixir_without_comments(text)
    apps = [m.group("name") for m in MIX_APP_DECLARATION.finditer(source)]
    if len(apps) != 1:
        raise ValueError(f"{manifest}: expected exactly one literal app name")
    deps: list[Dep] = []
    for match in MIX_DEPENDENCY_DECLARATION.finditer(source):
        body = match.group("body")
        path = MIX_PATH_OPTION.search(body)
        if path:
            deps.append(Dep(match.group("name"), None, path.group("path"), False, False))
            continue
        if _NON_HEX_SOURCE.search(body):
            continue
        leading = _LEADING_STRING.match(body)
        if not leading:
            continue  # not a dependency tuple (e.g. a stray {:ok, ...})
        deps.append(
            Dep(
                match.group("name"),
                leading.group("req"),
                None,
                bool(_ONLY_OPTION.search(body)),
                bool(_OPTIONAL_OPTION.search(body)),
            )
        )
    return Project(manifest.parent.resolve(), apps[0], deps)


@dataclass(frozen=True)
class LockEntry:
    name: str
    version: str
    inner: str
    outer: str | None


def parse_lock(text: str) -> tuple[dict[str, LockEntry], list[str]]:
    """Return ``(entries, problems)`` for a ``mix.lock`` body.

    A line that says ``:hex`` but does not match the expected tuple shape is a
    problem rather than being silently skipped.
    """
    entries: dict[str, LockEntry] = {}
    problems: list[str] = []
    for line in text.splitlines():
        if "{:hex," not in line:
            continue
        match = _LOCK_HEX.match(line)
        if not match:
            problems.append(f"unparseable :hex lock entry: {line.strip()[:60]}")
            continue
        name = match.group("name")
        entries[name] = LockEntry(
            name, match.group("version"), match.group("inner"), match.group("outer")
        )
        for label, value in (("inner", match.group("inner")), ("outer", match.group("outer"))):
            if value is None or not _SHA256.match(value):
                problems.append(f"{name}: missing or malformed {label} checksum")
    return entries, problems


# ---------------------------------------------------------------------------
# Repository model
# ---------------------------------------------------------------------------


def git_lines(root: Path, *args: str) -> list[str]:
    out = subprocess.run(
        ["git", *args], cwd=root, check=True, capture_output=True, text=True
    ).stdout
    return [line for line in out.splitlines() if line]


def load_projects(root: Path) -> dict[Path, Project]:
    projects: dict[Path, Project] = {}
    for rel in git_lines(root, "ls-files", ":(glob)**/mix.exs"):
        manifest = root / rel
        project = parse_mix_exs(manifest.read_text(encoding="utf-8"), manifest)
        projects[project.directory] = project
    return projects


def closure(
    project: Project, projects: dict[Path, Project]
) -> tuple[list[Project], dict[str, set[str]]]:
    """Return the path-dep closure and the Hex requirements it contributes.

    The result maps package name to the set of requirement strings that apply.
    """
    seen: dict[Path, Project] = {}
    hex_reqs: dict[str, set[str]] = {}
    stack = [project]
    while stack:
        current = stack.pop()
        if current.directory in seen:
            continue
        seen[current.directory] = current
        is_root = current is project
        for dep in current.deps:
            if dep.path is not None:
                target = (current.directory / dep.path).resolve()
                if target in projects:
                    stack.append(projects[target])
            elif is_root or not (dep.test_scoped or dep.optional):
                assert dep.requirement is not None
                hex_reqs.setdefault(dep.name, set()).add(dep.requirement)
    return list(seen.values()), hex_reqs


def find_roots(
    root: Path, projects: dict[Path, Project]
) -> dict[Path, dict[str, set[str]]]:
    """Map each root project directory to its Hex requirements."""
    roots: dict[Path, dict[str, set[str]]] = {}
    for directory, project in sorted(projects.items()):
        _, reqs = closure(project, projects)
        if reqs:
            roots[directory] = reqs
    return roots


def affected_directories(
    root: Path, projects: dict[Path, Project], ref: str
) -> set[Path] | None:
    """Directories of projects touched by ``git diff REF...HEAD``.

    Returns ``None`` (meaning "check everything") when this tool or its tests
    changed.
    """
    changed = [root / p for p in git_lines(root, "diff", "--name-only", f"{ref}...HEAD")]
    if any(p.name in {"elixir_locks.py", "test_elixir_locks.py"} for p in changed):
        return None
    affected: set[Path] = set()
    for directory, project in projects.items():
        members, _ = closure(project, projects)
        if any(
            path == member.directory or member.directory in path.parents
            for member in members
            for path in changed
        ):
            affected.add(directory)
    return affected


# ---------------------------------------------------------------------------
# The check
# ---------------------------------------------------------------------------


def run_check(root: Path, changed_since: str | None = None) -> list[str]:
    """Return a list of human-readable problems (empty means success)."""
    projects = load_projects(root)
    roots = find_roots(root, projects)
    tracked_locks = {
        (root / rel).parent.resolve(): root / rel
        for rel in git_lines(root, "ls-files", ":(glob)**/mix.lock")
    }
    scope = None
    if changed_since:
        scope = affected_directories(root, projects, changed_since)

    problems: list[str] = []
    versions: dict[str, dict[str, set[str]]] = {}  # package -> version -> lock dirs

    def rel(path: Path) -> str:
        return path.relative_to(root.resolve()).as_posix()

    for directory in sorted(set(roots) | set(tracked_locks)):
        if scope is not None and directory not in scope:
            continue
        lock_path = tracked_locks.get(directory)
        if lock_path is None:
            problems.append(f"{rel(directory)}: reaches Hex deps {sorted(roots[directory])} but has no tracked mix.lock")
            continue
        entries, lock_problems = parse_lock(lock_path.read_text(encoding="utf-8"))
        problems.extend(f"{rel(lock_path)}: {p}" for p in lock_problems)
        for entry in entries.values():
            versions.setdefault(entry.name, {}).setdefault(entry.version, set()).add(rel(lock_path))
        for name, requirements in sorted(roots.get(directory, {}).items()):
            entry = entries.get(name)
            if entry is None:
                problems.append(f"{rel(lock_path)}: missing entry for Hex dep {name!r}")
                continue
            for requirement in sorted(requirements):
                try:
                    ok = satisfies(requirement, entry.version)
                except ValueError as error:
                    problems.append(f"{rel(directory)}: {name}: {error}")
                    continue
                if not ok:
                    problems.append(
                        f"{rel(lock_path)}: {name} {entry.version} does not satisfy {requirement!r}"
                    )

    for name, by_version in sorted(versions.items()):
        if len(by_version) > 1:
            detail = "; ".join(
                f"{v} in {len(paths)} lock(s), e.g. {sorted(paths)[0]}"
                for v, paths in sorted(by_version.items())
            )
            problems.append(f"locks disagree on {name}: {detail}")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("roots", help="list projects whose closure includes a Hex dep")
    check = sub.add_parser("check", help="statically verify mix.lock coverage")
    check.add_argument("--changed-since", metavar="REF")
    args = parser.parse_args(argv)
    root = args.root.resolve()

    if args.command == "roots":
        for directory, reqs in find_roots(root, load_projects(root)).items():
            print(f"{directory.relative_to(root).as_posix()}: {', '.join(sorted(reqs))}")
        return 0

    problems = run_check(root, args.changed_since)
    for problem in problems:
        print(f"ERROR: {problem}", file=sys.stderr)
    if problems:
        print(f"{len(problems)} Elixir lock problem(s)", file=sys.stderr)
        return 1
    print("Elixir locks OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
