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
main is by then.  So, in a PR that changes the results file (rule 1's per-commit
detection, merge discounting included: a merge that merely brings main's newer
file does not count, and a deletion does), if the results file at ``--tree-rev`` (default ``HEAD``) records tree ids -- the
``trees`` object of its ``tool`` block (the merged-in run records of
``merges[i]`` are historical and are not read), mapping a name to a 40-hex id -- every id must equal
``git rev-parse <rev>:<path>`` for that name's path (the same entries as
``TREE_ENTRIES`` in the probe's ``src/meta.rs``: ``src/`` and ``Cargo.toml`` of the probe and the four
solver crates, the probe's ``Cargo.lock`` and the four files it includes at compile time; not
tests, READMEs or CHANGELOGs, which cannot change a recorded value).  A file without trees passes
with a note (the probe's ``check --full`` is what refuses such a file).  On a
pull request CI checks out ``refs/pull/N/merge`` (main plus the PR, the default
of ``actions/checkout``), which is exactly the tree that would land.  If the
file has a top-level ``recording`` block (every CI-recorded file does), then
``tool.trees`` MUST exist and hold exactly those names: omitting or trimming
the trees cannot switch the rule off (exit 1).  A bad JSON file (including a
duplicate key anywhere), a non-hex id, an unknown name in a file without a
``recording`` block, or a git failure exits 2.

Signed files (VOS-028c3c-3b-1b, VOS-D141).  A verify at the checkout runs at a tree that may
differ from the recorded one, and ``sign`` records that tree in the banner as
``regeneration.verified.trees`` (same shape as ``tool.trees``).  A file whose ``tool.trees``
are stale because a shared crate moved after the recording is therefore still acceptable when
the verify it cites ran at the tree that merges: rule 2 passes when EITHER ``tool.trees`` OR
``regeneration.verified.trees`` equals the merge ref's ids.  The recorded ``tool.trees`` are never
edited (they record where the file was recorded), and the ratchet and rule 1 are unchanged.
``verified.trees`` is validated exactly as strictly as ``tool.trees``: the exact set of entries
of ``TREE_PATHS`` (no fewer, no more) and 40-hex ids.  A ``regeneration`` that is not an object,
or a ``verified`` that is not an object holding such ``trees``, exits 2 (fail closed): a
malformed banner is not a reason to guess.  An unsigned file (``regeneration`` null or absent)
behaves exactly as before.

Scope.  Rule 2 and the ratchet apply ONLY to a PR whose range changes the
results file.  A required PR gate on "recorded trees equal the checked-out
trees" would fail every PR anywhere in the repository that touches the probe or
the build inputs of the four SHARED solver crates (bit-blaster, cdcl-sat, drat-check,
riscv-simulator) once a recording exists, freezing unrelated work until a full
re-record.  Drift of the trees on main is the job of the gated verify of VOS30
section 9.3: it runs when the tree ids change, goes red, and a re-record
follows.  Other PRs pass with a note.

Ratchet: a recorded file cannot be downgraded.  If the results file at BASE
records tree ids or has a ``recording`` block, the checked-out file must still
hold all the trees; dropping both the block and the trees, or deleting the
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
# Mirrors TREE_ENTRIES of the probe (src/meta.rs): one git id per BUILD INPUT, not per crate
# (VOS-D139, VOS-D140).  tests/test_vos_results_guard.py parses meta.rs and asserts equality, so
# the two cannot drift.  Name -> repository path; `git rev-parse <rev>:<path>` is a tree id for a
# directory and a blob id for a file.
TREE_PATHS = {
    "vos-capacity-probe/src": "code/programs/rust/vos-capacity-probe/src",
    "vos-capacity-probe/Cargo.toml": "code/programs/rust/vos-capacity-probe/Cargo.toml",
    "vos-capacity-probe/Cargo.lock": "code/programs/rust/vos-capacity-probe/Cargo.lock",
    "bit-blaster/src": "code/packages/rust/bit-blaster/src",
    "bit-blaster/Cargo.toml": "code/packages/rust/bit-blaster/Cargo.toml",
    "cdcl-sat/src": "code/packages/rust/cdcl-sat/src",
    "cdcl-sat/Cargo.toml": "code/packages/rust/cdcl-sat/Cargo.toml",
    "drat-check/src": "code/packages/rust/drat-check/src",
    "drat-check/Cargo.toml": "code/packages/rust/drat-check/Cargo.toml",
    "riscv-simulator/src": "code/packages/rust/riscv-simulator/src",
    "riscv-simulator/Cargo.toml": "code/packages/rust/riscv-simulator/Cargo.toml",
    "vos-sched-fp-bitmap/tests/generated/warrant_evidence.rs":
        "code/packages/rust/vos-sched-fp-bitmap/tests/generated/warrant_evidence.rs",
    "vos-probe-ready/tests/generated/warrant_evidence.rs":
        "code/packages/rust/vos-probe-ready/tests/generated/warrant_evidence.rs",
    "specs/vos-sched-fp-bitmap.warrant": "code/specs/vos-sched-fp-bitmap.warrant",
    "specs/vos-probe-ready.warrant": "code/specs/vos-probe-ready.warrant",
    "arithmetic/src": "code/packages/rust/arithmetic/src",
    "arithmetic/Cargo.toml": "code/packages/rust/arithmetic/Cargo.toml",
    "bounded-json/src": "code/packages/rust/bounded-json/src",
    "bounded-json/Cargo.toml": "code/packages/rust/bounded-json/Cargo.toml",
    "sha256/src": "code/packages/rust/sha256/src",
    "sha256/Cargo.toml": "code/packages/rust/sha256/Cargo.toml",
    "test-bootstrap/src": "code/packages/rust/test-bootstrap/src",
    "test-bootstrap/Cargo.toml": "code/packages/rust/test-bootstrap/Cargo.toml",
    "warrant-grammar/src": "code/packages/rust/warrant-grammar/src",
    "warrant-grammar/Cargo.toml":
        "code/packages/rust/warrant-grammar/Cargo.toml",
    "zeroize/src": "code/packages/rust/zeroize/src",
    "zeroize/Cargo.toml": "code/packages/rust/zeroize/Cargo.toml",
    "cpu-simulator/src": "code/packages/rust/cpu-simulator/src",
    "cpu-simulator/Cargo.toml": "code/packages/rust/cpu-simulator/Cargo.toml",
    "grammar-tools/src": "code/packages/rust/grammar-tools/src",
    "grammar-tools/Cargo.toml": "code/packages/rust/grammar-tools/Cargo.toml",
    "logic-gates/src": "code/packages/rust/logic-gates/src",
    "logic-gates/Cargo.toml": "code/packages/rust/logic-gates/Cargo.toml",
    "riscv-encoder/src": "code/packages/rust/riscv-encoder/src",
    "riscv-encoder/Cargo.toml": "code/packages/rust/riscv-encoder/Cargo.toml",
    "transistors/src": "code/packages/rust/transistors/src",
    "transistors/Cargo.toml": "code/packages/rust/transistors/Cargo.toml",
    "vos-probe-ready/src": "code/packages/rust/vos-probe-ready/src",
    "vos-probe-ready/Cargo.toml":
        "code/packages/rust/vos-probe-ready/Cargo.toml",
    "vos-sched-contract/src": "code/packages/rust/vos-sched-contract/src",
    "vos-sched-contract/Cargo.toml":
        "code/packages/rust/vos-sched-contract/Cargo.toml",
    "vos-sched-fp-bitmap/src": "code/packages/rust/vos-sched-fp-bitmap/src",
    "vos-sched-fp-bitmap/Cargo.toml":
        "code/packages/rust/vos-sched-fp-bitmap/Cargo.toml",
    "test-bootstrap/tests/exit_fixture.rs":
        "code/packages/rust/test-bootstrap/tests/exit_fixture.rs",
    "test-bootstrap/tests/self_corpus.rs":
        "code/packages/rust/test-bootstrap/tests/self_corpus.rs",
    "repo/.cargo/config.toml": ".cargo/config.toml",
}
REV_RE = re.compile(r"^[0-9A-Za-z_][0-9A-Za-z._/~^-]*$")
HEX40 = re.compile(r"^[0-9a-f]{40}$")
MAX_COMMITS = 500
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
RUN_NUMBER = re.compile(r"^[0-9]{1,20}$")
# Every pattern is applied with fullmatch: `$` alone also matches before a trailing newline, so
# an id followed by a newline must not pass as an id (VOS-028c3c-3b-1b hardening).


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
    """Return rule 1's violations (empty when the range passes)."""
    return analyse(repo, base, head)[0]


def analyse(repo: str, base: str, head: str) -> tuple[list[str], bool]:
    """Rule 1: (violations, whether the range changes the results file)."""
    for name, sha in (("--base", base), ("--head", head)):
        if not SHA_RE.fullmatch(sha):
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
        return [], False
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
    return problems, True


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


def verified_trees_of(doc: dict):
    """``regeneration.verified.trees`` of a signed file, or None when there is none.

    Validated as strictly as ``tool.trees`` (VOS-028c3c-3b-1b): exactly the entries of
    ``TREE_PATHS`` and 40-hex ids; anything else is malformed and exits 2.
    """
    reg = doc.get("regeneration")
    if reg is None:
        return None
    if not isinstance(reg, dict):
        raise GuardError("regeneration is not an object")
    # A signed file always carries `verified` (the probe refuses one without it), so its absence
    # is malformed here too: fail closed rather than fall back to tool.trees alone.
    if "verified" not in reg:
        raise GuardError("regeneration has no verified block")
    ver = reg["verified"]
    if not isinstance(ver, dict):
        raise GuardError("regeneration.verified is not an object")
    # The banner fields around the trees are held to the probe's shapes before the trees are
    # trusted: lower-case hex digests and revisions, digit run numbers.
    for holder, label, key, pattern in (
        (reg, "regeneration", "unsigned_sha256", HEX64),
        (ver, "regeneration.verified", "verify_sha256", HEX64),
        (ver, "regeneration.verified", "head_sha", HEX40),
        (ver, "regeneration.verified", "run_id", RUN_NUMBER),
        (ver, "regeneration.verified", "run_attempt", RUN_NUMBER),
    ):
        value = holder.get(key)
        if not isinstance(value, str) or not pattern.fullmatch(value):
            raise GuardError(f"{label}.{key} is missing or malformed")
    vtrees = ver.get("trees")
    if not isinstance(vtrees, dict):
        raise GuardError("regeneration.verified.trees is not an object")
    if set(vtrees) != set(TREE_PATHS):
        raise GuardError(
            "regeneration.verified.trees must hold exactly "
            f"{sorted(TREE_PATHS)}; it has {sorted(vtrees)}"
        )
    for name, tid in vtrees.items():
        if not isinstance(tid, str) or not HEX40.fullmatch(tid):
            raise GuardError(f"regeneration.verified.trees[{name!r}] is not a 40-hex id")
    return vtrees


def compare_trees(repo: str, rev: str, label: str, trees: dict, fix: str) -> list[str]:
    """The ids of ``trees`` that differ from ``git rev-parse <rev>:<path>``, one line each."""
    problems = []
    for name, tid in trees.items():
        if name not in TREE_PATHS:
            raise GuardError(f"{label} names unknown tree {name!r}")
        if not isinstance(tid, str) or not HEX40.fullmatch(tid):
            raise GuardError(f"{label}[{name!r}] is not a 40-hex id")
        path = TREE_PATHS[name]
        actual = git(repo, "rev-parse", "--verify", "--end-of-options", f"{rev}:{path}").strip()
        if actual != tid:
            problems.append(
                f"{label}[{name}] records {tid} but {path} is {actual} at {rev}; "
                f"{fix} {path}"
            )
    return problems


def check_trees(repo: str, rev: str, base: str | None = None) -> tuple[list[str], str]:
    """Rule 2 and the ratchet: (violations, note)."""
    if not REV_RE.fullmatch(rev):
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
    problems = compare_trees(
        repo, rev, "tool.trees", trees, "re-record, or revert the change to"
    )
    # A signed file may instead have been verified at the tree that merges (VOS-D141).  Read and
    # validated every time, so a malformed banner exits 2 even when tool.trees already match.
    vtrees = verified_trees_of(doc)
    if vtrees is not None and problems:
        vproblems = compare_trees(
            repo, rev, "regeneration.verified.trees", vtrees,
            "verify again at this tree, or revert the change to",
        )
        if not vproblems:
            return [], ""
        problems += vproblems
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
        problems, touched = analyse(args.repo, args.base, args.head)
        if touched:
            tree_problems, note = check_trees(args.repo, args.tree_rev, args.base)
            problems += tree_problems
        else:
            note = (
                "this PR does not change the results file; tree drift is the verify "
                "workflow's job, VOS30 9.3"
            )
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
