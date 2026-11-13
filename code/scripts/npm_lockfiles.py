#!/usr/bin/env python3
"""Verify that every npm install in the repo is a locked, reproducible install.

Why this exists
---------------
``npm install`` re-resolves the dependency tree: if a lock file is missing,
stale, or simply older than a newly published release, it quietly picks
whatever the registry offers today (and may rewrite the lock).  ``npm ci`` is
the opposite: it installs exactly what ``package-lock.json`` records and
FAILS if the lock is absent or disagrees with ``package.json``.  A build that
must be reproducible therefore needs three things, and this script enforces
all of them without running npm:

1. **No ``npm install`` in BUILD files.**  Every tracked ``BUILD`` and
   ``BUILD_windows`` line must use ``npm ci``.  The only exceptions are
   installing npm itself (``npm install --global npm@<exact>``) and one-off
   tool installs that name an exact version
   (``npm install --no-save <pkg>@<exact>``).

2. **A lock file wherever ``npm ci`` runs.**  We follow the ``cd`` commands in
   each BUILD line (``cd ../dep && npm ci``, ``(cd ../dep && npm ci)``,
   ``npm ci --prefix ../dep``) to find every directory that gets installed.
   If that directory has a tracked ``package.json`` it must also have a
   tracked ``package-lock.json``.  (Directories with no ``package.json`` at
   all, such as a Python package whose BUILD installs TypeScript siblings, are
   skipped: nothing is installed there.)

3. **Locks that are in sync and well-sourced.**  For every tracked lock that
   sits beside a ``package.json``:

   * the lock's root entry (``packages[""]``) must list the same
     ``dependencies`` / ``devDependencies`` / ``optionalDependencies`` /
     ``peerDependencies`` specs as ``package.json``.  This is precisely the
     comparison ``npm ci`` makes before it installs anything, so a mismatch
     here is a guaranteed CI failure we can report early and cheaply;
   * every ``resolved`` URL must be on ``https://registry.npmjs.org/`` (or be
     a relative ``file:`` style path for sibling packages), and every registry
     package must carry an ``integrity`` hash, so the install is content
     addressed and cannot be redirected to another host.

Usage::

    python3 code/scripts/npm_lockfiles.py            # check the whole repo
    python3 code/scripts/npm_lockfiles.py --root DIR # check another checkout

Exit status is 0 when clean and 1 when any problem is found.
"""

from __future__ import annotations

import argparse
import json
import posixpath
import re
import subprocess
import sys
from pathlib import Path

REGISTRY_PREFIX = "https://registry.npmjs.org/"

# The four sections of package.json that npm ci checks against the lock root.
DEPENDENCY_SECTIONS = (
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
)

# "@1.2.3" or "@1.2.3-rc.1": an exact version, never a range or a tag.
EXACT_VERSION = re.compile(r"@\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?(?:\+[0-9A-Za-z.]+)?$")

# `npm install`, `npm i`, `npm.cmd install` ... plus everything up to the next
# shell operator, so the flags and package arguments can be inspected.
NPM_COMMAND = re.compile(
    r"\bnpm(?:\.cmd)?\s+(?P<verb>install|i|ci)\b(?P<args>[^&|;()\n]*)"
)

_QUOTED = re.compile(r'"[^"]*"|\'[^\']*\'')
_SEPARATORS = re.compile(r"(&&|\|\||;|\(|\))")


# --------------------------------------------------------------------------
# BUILD file analysis
# --------------------------------------------------------------------------


def _is_allowed_install(args: str) -> bool:
    """True for the two sanctioned uses of ``npm install`` in a BUILD file."""
    tokens = args.split()
    # npm install --global npm@11.13.0  (pinning npm itself)
    if ("--global" in tokens or "-g" in tokens) and tokens:
        packages = [t for t in tokens if not t.startswith("-")]
        return bool(packages) and all(EXACT_VERSION.search(p) for p in packages)
    # npm install --no-save <pkg>@<exact>  (one-off tool, exactly pinned)
    if "--no-save" in tokens:
        packages = [t for t in tokens if not t.startswith("-")]
        return bool(packages) and all(EXACT_VERSION.search(p) for p in packages)
    return False


def _resolve(base: str, target: str) -> str:
    """Join *target* onto *base* (both repo-relative, POSIX) and normalise."""
    return posixpath.normpath(posixpath.join(base, target))


def scan_build_line(build_dir: str, line: str) -> tuple[list[str], list[str]]:
    """Analyse one BUILD line.

    Returns ``(bad_installs, ci_dirs)``: the offending ``npm install``
    fragments, and the repo-relative directories where ``npm ci`` runs.

    A BUILD line runs in a fresh shell starting at *build_dir*.  We walk its
    ``&&``/``;``-separated commands tracking ``cd``, with ``( ... )`` opening a
    subshell whose ``cd`` does not leak out, which is how the repo writes
    ``(cd ../dep && npm ci)``.
    """
    bad: list[str] = []
    ci_dirs: list[str] = []

    # Windows BUILD_windows files use backslashes; quoted text (powershell
    # snippets, "$(find ...)") must not influence parenthesis tracking.
    text = line.replace("\\", "/")
    text = _QUOTED.sub('""', text)

    stack = [build_dir]
    previous = build_dir  # what `cd -` returns to
    for piece in _SEPARATORS.split(text):
        piece = piece.strip()
        if not piece:
            continue
        if piece == "(":
            stack.append(stack[-1])
            continue
        if piece == ")":
            if len(stack) > 1:
                stack.pop()
            continue
        if piece in ("&&", "||", ";"):
            continue

        tokens = piece.split()
        # `cd`, `cd /d X` (cmd.exe), `cd -`
        if tokens and tokens[0] == "cd":
            args = [t for t in tokens[1:] if t.lower() != "/d"]
            if args:
                before = stack[-1]
                if args[0] == "-":
                    stack[-1] = previous
                else:
                    stack[-1] = _resolve(stack[-1], args[0].strip('"'))
                previous = before
            continue

        for match in NPM_COMMAND.finditer(piece):
            verb = match.group("verb")
            args = match.group("args")
            if verb in ("install", "i"):
                if not _is_allowed_install(args):
                    bad.append(f"npm {verb}{args}".rstrip())
                continue
            # npm ci: the target is --prefix <dir> when given, else the cwd.
            target = stack[-1]
            tokens_ci = args.split()
            for i, tok in enumerate(tokens_ci):
                if tok == "--prefix" and i + 1 < len(tokens_ci):
                    target = _resolve(stack[-1], tokens_ci[i + 1].strip('"'))
                elif tok.startswith("--prefix="):
                    target = _resolve(stack[-1], tok.split("=", 1)[1].strip('"'))
            ci_dirs.append(target)
    return bad, ci_dirs


def scan_build_file(path: str, text: str) -> tuple[list[str], set[str]]:
    """Analyse a whole BUILD file; *path* is its repo-relative POSIX path."""
    build_dir = posixpath.dirname(path) or "."
    problems: list[str] = []
    ci_dirs: set[str] = set()
    for number, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith("#"):
            continue
        bad, dirs = scan_build_line(build_dir, line)
        for fragment in bad:
            problems.append(
                f"{path}:{number}: `{fragment}` must be `npm ci` "
                "(only `--global npm@<exact>` and `--no-save <pkg>@<exact>` "
                "may use npm install)"
            )
        ci_dirs.update(dirs)
    return problems, ci_dirs


# --------------------------------------------------------------------------
# Lock file analysis
# --------------------------------------------------------------------------


def check_lock_matches_package(lock: dict, package: dict, label: str) -> list[str]:
    """Compare the lock's root entry with package.json, as ``npm ci`` does."""
    root = lock.get("packages", {}).get("")
    if root is None:
        return [f"{label}: lock has no root packages[\"\"] entry"]
    problems: list[str] = []
    for section in DEPENDENCY_SECTIONS:
        want = package.get(section) or {}
        have = root.get(section) or {}
        if want == have:
            continue
        for name in sorted(set(want) | set(have)):
            if want.get(name) == have.get(name):
                continue
            problems.append(
                f"{label}: {section}[{name}] is "
                f"{want.get(name, '<absent>')!r} in package.json but "
                f"{have.get(name, '<absent>')!r} in the lock; run `npm install "
                "--package-lock-only` to refresh it"
            )
    return problems


def check_lock_sources(lock: dict, label: str) -> list[str]:
    """Every download must come from the npm registry and carry integrity."""
    problems: list[str] = []
    for key, entry in sorted(lock.get("packages", {}).items()):
        if not key:
            continue
        resolved = entry.get("resolved")
        if resolved is None:
            continue
        if resolved.startswith(REGISTRY_PREFIX):
            if not entry.get("integrity"):
                problems.append(f"{label}: {key} has no integrity hash")
        elif "://" in resolved or resolved.startswith(("git+", "github:")):
            problems.append(
                f"{label}: {key} resolves outside the npm registry: {resolved}"
            )
        # else: a relative path ("../sibling") for a file: dependency.
    return problems


# --------------------------------------------------------------------------
# Whole-repo driver
# --------------------------------------------------------------------------


def tracked_files(root: Path) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
    ).stdout
    return [f for f in out.decode("utf-8").split("\0") if f]


def check_repo(root: Path, files: list[str]) -> list[str]:
    tracked = set(files)
    problems: list[str] = []
    ci_dirs: set[str] = set()

    for path in files:
        if posixpath.basename(path) in ("BUILD", "BUILD_windows"):
            text = (root / path).read_text(encoding="utf-8", errors="replace")
            found, dirs = scan_build_file(path, text)
            problems.extend(found)
            ci_dirs.update(dirs)

    for directory in sorted(ci_dirs):
        prefix = "" if directory == "." else directory + "/"
        if prefix + "package.json" in tracked and prefix + "package-lock.json" not in tracked:
            problems.append(
                f"{prefix}package-lock.json: missing, but a BUILD runs `npm ci` "
                f"in {directory}"
            )

    for path in files:
        if posixpath.basename(path) != "package-lock.json":
            continue
        directory = posixpath.dirname(path)
        prefix = directory + "/" if directory else ""
        lock = json.loads((root / path).read_text(encoding="utf-8"))
        problems.extend(check_lock_sources(lock, path))
        if prefix + "package.json" in tracked:
            package = json.loads((root / (prefix + "package.json")).read_text("utf-8"))
            problems.extend(check_lock_matches_package(lock, package, path))
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args(argv)

    problems = check_repo(args.root, tracked_files(args.root))
    if problems:
        print(f"npm lockfile check failed ({len(problems)} problem(s)):")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("npm lockfile check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
