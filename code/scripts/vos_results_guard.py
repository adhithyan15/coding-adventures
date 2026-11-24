#!/usr/bin/env python3
"""Guard: commits after the capacity recording touch only ``code/specs/**`` (VOS-D126).

Why this exists
===============

``code/specs/data/vos30-capacity-results.json`` records the git tree ids of the
capacity probe and its four solver crates.  VOS-028c3c lands as ONE pull request
with two commits:

    commit A   probe activation (the code the recording measures)
    commit B   the recorded results file, plus other ``code/specs/**`` edits

Any commit *after* the recording that touches a path outside ``code/specs/**``
could change those trees, so ``check --full`` would refuse the file as stale --
or, worse, a reviewer could be shown a file recorded at a different tree than
the one that merges.  A rule over the whole PR ("if the results file changes,
only ``code/specs/**`` changes") would fail the very PR it protects, because
commit A is in it.  So the rule is per commit:

    Let R be the LAST commit in BASE..HEAD that changes the results file.
    R and every later commit may change only paths under ``code/specs/**``.

A range that never changes the results file passes at once.

Merge commits
=============

A branch often takes ``git merge origin/main``.  Only the first-parent chain is
walked, so the commits that main contributed are not judged one by one.  For a
merge commit M with parents P1 (this branch) and P2 (the merged-in side), the
paths M changes *relative to both parents* are the PR's own contribution (the
conflict resolutions); a path M took unchanged from P2 is main's content, not an
edit by this PR, and is ignored:

    own(M) = changed(P1, M)  intersect  changed(P2, M)

A merge that brings a newer results file from main is therefore not "the commit
that changes the results file".  Octopus merges use the first parent and every
other parent in the same way.

Where it runs
=============

``ci.yml`` metadata-contracts job, on ``pull_request`` only, with the PR's base
and head SHAs.  It does not run on a push to ``main``: the repository
squash-merges, so the pushed commit holds A and B together and the per-commit
rule cannot hold there; the PR is the gate, and ``check --full`` still compares
the recorded tree ids with the merged tree.

Exit status: 0 pass, 1 violation, 2 usage or git error (fail closed).
No shell is used; git is run with a fixed argv and SHAs must be 40 hex digits.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys

RESULTS_FILE = "code/specs/data/vos30-capacity-results.json"
ALLOWED_PREFIX = "code/specs/"
SHA_RE = re.compile(r"^[0-9a-f]{40}$")


class GuardError(Exception):
    """A usage or git failure (exit status 2)."""


def git(repo: str, *args: str) -> str:
    try:
        proc = subprocess.run(
            ["git", "-C", repo, *args],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )
    except OSError as exc:
        raise GuardError(f"cannot run git: {exc}") from exc
    if proc.returncode != 0:
        raise GuardError(f"git {' '.join(args)} failed: {proc.stderr.strip()}")
    return proc.stdout


def changed(repo: str, a: str, b: str) -> set[str]:
    out = git(repo, "diff", "--name-only", "--no-renames", "-z", a, b)
    return {p for p in out.split("\0") if p}


def own_changes(repo: str, commit: str) -> set[str]:
    """Paths the commit itself contributes (see "Merge commits" above)."""
    parents = git(repo, "rev-list", "--parents", "-n", "1", commit).split()[1:]
    if not parents:
        raise GuardError(f"commit {commit} has no parent")
    paths = changed(repo, parents[0], commit)
    for other in parents[1:]:
        paths &= changed(repo, other, commit)
    return paths


def outside_specs(paths: set[str]) -> list[str]:
    return sorted(p for p in paths if not p.startswith(ALLOWED_PREFIX))


def check(repo: str, base: str, head: str) -> list[str]:
    """Return the violations (empty when the range passes)."""
    for name, sha in (("--base", base), ("--head", head)):
        if not SHA_RE.match(sha):
            raise GuardError(f"{name} must be a 40-hex commit SHA, got {sha!r}")
    commits = git(
        repo, "rev-list", "--first-parent", "--reverse", f"{base}..{head}"
    ).split()
    own = [(c, own_changes(repo, c)) for c in commits]
    last = None
    for i, (_, paths) in enumerate(own):
        if RESULTS_FILE in paths:
            last = i
    if last is None:
        return []
    problems = []
    for i in range(last, len(own)):
        commit, paths = own[i]
        bad = outside_specs(paths)
        if not bad:
            continue
        if i == last:
            why = (
                f"changes {RESULTS_FILE} together with paths outside code/specs/**"
                " (the recording commit must be specs-only)"
            )
        else:
            why = (
                f"comes after the commit that changes {RESULTS_FILE} "
                f"({own[last][0][:12]}) but touches paths outside code/specs/**"
            )
        problems.append(f"commit {commit[:12]} {why}: " + ", ".join(bad))
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--base", required=True, help="40-hex base commit (exclusive)")
    parser.add_argument("--head", required=True, help="40-hex head commit (inclusive)")
    parser.add_argument("--repo", default=".", help="repository directory")
    try:
        args = parser.parse_args(argv)
    except SystemExit as exc:
        return 2 if exc.code else 0
    try:
        problems = check(args.repo, args.base, args.head)
    except GuardError as exc:
        print(f"vos_results_guard: error: {exc}", file=sys.stderr)
        return 2
    if problems:
        for line in problems:
            print(f"vos_results_guard: VIOLATION: {line}", file=sys.stderr)
        return 1
    print("vos_results_guard: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
