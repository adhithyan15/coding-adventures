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
that changes the results file".

The discount applies ONLY when every non-first parent is already contained in
BASE (``git merge-base --is-ancestor P BASE``): that is what "taken from main"
means.  Otherwise (a side branch of the PR itself, which could carry a probe
edit made after the recording) nothing is discounted and the merge is judged by
its full diff against its first parent.  Octopus merges follow the same rule.

Rule 2: the recorded trees equal the checked-out tree
=====================================================

Rule 1 cannot see the recorded trees going stale through MAIN: a merge from
main takes its paths unchanged (ignored above), and a squash lands on whatever
main is by then.  So, whether or not the range touches the results file, if the
results file at ``--tree-rev`` (default ``HEAD``) records tree ids -- the
``trees`` object of its ``tool`` block (the merged-in run records of
``merges[i]`` are historical and are not read), mapping a name to a 40-hex id -- every id must equal
``git rev-parse <rev>:<path>`` for that name's path (the same five paths as
``TREE_PATHS`` in the probe's ``src/meta.rs``).  A file without trees passes
with a note (the probe's ``check --full`` is what refuses such a file).  On a
pull request CI checks out ``refs/pull/N/merge`` (main plus the PR, the default
of ``actions/checkout``), which is exactly the tree that would land.  If the
file has a top-level ``recording`` block (every CI-recorded file does), then
``tool.trees`` MUST exist and hold exactly the five names: omitting or trimming
the trees cannot switch the rule off (exit 1).  A bad JSON file (including a
duplicate key anywhere), a non-hex id, an unknown name in a file without a
``recording`` block, or a git failure exits 2.

Ratchet: a recorded file cannot be downgraded.  If the results file at BASE
records tree ids or has a ``recording`` block, the checked-out file must still
hold all five trees; dropping both the block and the trees, or deleting the
file, exits 1 (deleting a recorded results file needs a spec change).  A file
that is absent or tree-less at BASE (the tool 0.1.0 file) has no ratchet.

Where it runs
=============

``ci.yml`` metadata-contracts job, on ``pull_request`` only, with the PR's base
and head SHAs.  It does not run on a push to ``main``: the repository
squash-merges, so the pushed commit holds A and B together and the per-commit
rule cannot hold there; the PR is the gate, and ``check --full`` still compares
the recorded tree ids with the merged tree.

Not covered here: a PR can edit ``ci.yml`` or this script itself, and the repository
has no CODEOWNERS.  That is the concern of the release gate G1 (CODEOWNERS and
branch protection, VOS-D026 and VOS-D067), not of this guard.

Hardening: git output is decoded with ``surrogateescape`` and any decode or
recursion error exits 2; revision arguments follow ``--end-of-options`` where
git accepts it (git >= 2.24); the first-parent range is capped at
``MAX_COMMITS`` (exit 2 beyond it).

Exit status: 0 pass, 1 violation, 2 usage or git error (fail closed).
No shell is used; git is run with a fixed argv and SHAs must be 40 hex digits.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys

RESULTS_FILE = "code/specs/data/vos30-capacity-results.json"
ALLOWED_PREFIX = "code/specs/"
# Mirrors TREE_NAMES / TREE_PATHS of the probe (src/results.rs, src/meta.rs).
TREE_PATHS = {
    "vos-capacity-probe": "code/programs/rust/vos-capacity-probe",
    "bit-blaster": "code/packages/rust/bit-blaster",
    "cdcl-sat": "code/packages/rust/cdcl-sat",
    "drat-check": "code/packages/rust/drat-check",
    "riscv-simulator": "code/packages/rust/riscv-simulator",
}
REV_RE = re.compile(r"^[0-9A-Za-z_][0-9A-Za-z._/~^-]*$")
HEX40 = re.compile(r"^[0-9a-f]{40}$")
MAX_COMMITS = 500
SHA_RE = re.compile(r"^[0-9a-f]{40}$")


class GuardError(Exception):
    """A usage or git failure (exit status 2)."""


def run_git(repo: str, *args: str) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(
            ["git", "-C", repo, *args], capture_output=True, check=False
        )
    except OSError as exc:
        raise GuardError(f"cannot run git: {exc}") from exc


def decode(raw: bytes) -> str:
    """Bytes to text without ever raising: undecodable bytes become surrogates."""
    return raw.decode("utf-8", errors="surrogateescape")


def printable(text: str) -> str:
    """Text safe to print: surrogates (undecodable bytes) become U+FFFD."""
    return text.encode("utf-8", errors="surrogateescape").decode("utf-8", errors="replace")


def git(repo: str, *args: str) -> str:
    proc = run_git(repo, *args)
    if proc.returncode != 0:
        raise GuardError(
            f"git {' '.join(args)} failed: {printable(decode(proc.stderr)).strip()}"
        )
    return decode(proc.stdout)


def changed(repo: str, a: str, b: str) -> set[str]:
    out = git(repo, "diff", "--name-only", "--no-renames", "-z", "--end-of-options", a, b)
    return {p for p in out.split("\0") if p}


def is_ancestor(repo: str, commit: str, base: str) -> bool:
    proc = run_git(repo, "merge-base", "--is-ancestor", "--end-of-options", commit, base)
    if proc.returncode not in (0, 1):
        raise GuardError(f"git merge-base --is-ancestor failed: {printable(decode(proc.stderr)).strip()}")
    return proc.returncode == 0


def own_changes(repo: str, commit: str, base: str) -> set[str]:
    """Paths the commit itself contributes (see "Merge commits" above)."""
    parents = git(repo, "rev-list", "--parents", "-n", "1", "--end-of-options", commit).split()[1:]
    if not parents:
        raise GuardError(f"commit {commit} has no parent")
    paths = changed(repo, parents[0], commit)
    others = parents[1:]
    if others and all(is_ancestor(repo, p, base) for p in others):
        for other in others:
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
        repo, "rev-list", "--first-parent", "--reverse",
        f"--max-count={MAX_COMMITS + 1}", "--end-of-options", f"{base}..{head}",
    ).split()
    if len(commits) > MAX_COMMITS:
        raise GuardError(f"more than {MAX_COMMITS} first-parent commits in the range")
    own = [(c, own_changes(repo, c, base)) for c in commits]
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
        problems.append(f"commit {commit[:12]} {why}: " + ", ".join(printable(p) for p in bad))
    return problems


def no_duplicate_keys(pairs):
    keys = [k for k, _ in pairs]
    if len(set(keys)) != len(keys):
        raise ValueError("duplicate JSON key")
    return dict(pairs)


def read_results(repo: str, rev: str):
    """The parsed results file at ``rev``, or None if the file is absent there."""
    if not git(repo, "ls-tree", "--full-tree", "--name-only", rev, "--", RESULTS_FILE).strip():
        return None
    try:
        doc = json.loads(
            git(repo, "show", f"{rev}:{RESULTS_FILE}"), object_pairs_hook=no_duplicate_keys
        )
    except ValueError as exc:
        raise GuardError(f"{RESULTS_FILE} at {rev} is not valid JSON: {exc}") from exc
    if not isinstance(doc, dict):
        raise GuardError(f"results file at {rev} is not a JSON object")
    return doc


def trees_of(doc: dict):
    tool = doc.get("tool")
    return tool.get("trees") if isinstance(tool, dict) else None


def is_recorded(doc) -> bool:
    return doc is not None and ("recording" in doc or bool(trees_of(doc)))


def check_trees(repo: str, rev: str, base: str | None = None) -> tuple[list[str], str]:
    """Rule 2 and the ratchet: (violations, note)."""
    if not REV_RE.match(rev):
        raise GuardError(f"--tree-rev {rev!r} is not a plain revision")
    doc = read_results(repo, rev)
    base_doc = read_results(repo, base) if base is not None else None
    if is_recorded(base_doc):
        if doc is None:
            return [
                f"{RESULTS_FILE} records tree ids at the PR base and this PR deletes it; "
                "deleting a recorded results file needs a spec change"
            ], ""
        t = trees_of(doc)
        if not (isinstance(t, dict) and set(t) == set(TREE_PATHS)):
            return [
                f"the results file at the PR base records tree ids; this PR's file does not "
                f"(downgrade): tool.trees must hold exactly {sorted(TREE_PATHS)}"
            ], ""
    if doc is None:
        return [], f"no {RESULTS_FILE} at {rev}"
    trees = trees_of(doc)
    if "recording" in doc:
        if not isinstance(trees, dict) or set(trees) != set(TREE_PATHS):
            have = sorted(trees) if isinstance(trees, dict) else trees
            return [
                f"{RESULTS_FILE} has a recording block, so tool.trees must hold exactly "
                f"{sorted(TREE_PATHS)}; it has {have}"
            ], ""
    elif trees is None:
        return [], "the results file records no trees; rule 2 has nothing to check"
    if not isinstance(trees, dict):
        raise GuardError("tool.trees is not an object")
    problems = []
    for name, tid in trees.items():
        if name not in TREE_PATHS:
            raise GuardError(f"tool.trees names unknown tree {name!r}")
        if not isinstance(tid, str) or not HEX40.match(tid):
            raise GuardError(f"tool.trees[{name!r}] is not a 40-hex id")
        path = TREE_PATHS[name]
        actual = git(repo, "rev-parse", "--verify", "--end-of-options", f"{rev}:{path}").strip()
        if actual != tid:
            problems.append(
                f"tool.trees[{name}] records {tid} but {path} is {actual} at {rev}; "
                f"re-record, or revert the change to {path}"
            )
    return problems, ""


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--base", required=True, help="40-hex base commit (exclusive)")
    parser.add_argument("--head", required=True, help="40-hex head commit (inclusive)")
    parser.add_argument("--tree-rev", default="HEAD", help="revision rule 2 reads (default HEAD)")
    parser.add_argument("--repo", default=".", help="repository directory")
    try:
        args = parser.parse_args(argv)
    except SystemExit as exc:
        return 2 if exc.code else 0
    try:
        problems = check(args.repo, args.base, args.head)
        tree_problems, note = check_trees(args.repo, args.tree_rev, args.base)
        problems += tree_problems
        if note:
            print(f"vos_results_guard: note: {note}")
    except (GuardError, UnicodeError, RecursionError) as exc:
        print(f"vos_results_guard: error: {printable(str(exc))}", file=sys.stderr)
        return 2
    if problems:
        for line in problems:
            print(f"vos_results_guard: VIOLATION: {line}", file=sys.stderr)
        return 1
    print("vos_results_guard: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
