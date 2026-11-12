#!/usr/bin/env python3
"""Keep every Cargo project's ``Cargo.lock`` committed and current.

Why this exists
===============

A ``Cargo.lock`` records the exact version (and checksum) of every crate a
project resolved.  Without a committed lock, two builds of the same commit may
silently pick different crate versions, and a registry release can break CI
with no change in the repository.  The repository used to ignore every
``Cargo.lock``; this script is the other half of reversing that: it finds
every place a lock belongs, creates the locks, and fails CI when one has gone
stale.

Vocabulary
----------

*Cargo root*
    A directory whose ``Cargo.toml`` either declares ``[workspace]`` or is not a
    member of any enclosing workspace.  Cargo writes exactly one ``Cargo.lock``
    per root, next to the root manifest.  Members of a workspace never get
    their own lock; they share the root's.

    A manifest below a workspace root is a *member* when

    1. it matches a ``members`` glob and no ``exclude`` entry, or
    2. it is not excluded and a member reaches it through a ``path``
       dependency (Cargo adds such crates to the workspace implicitly).

    Anything else is a root of its own (``os-kernel`` is excluded from the main
    workspace, and every detached ``*-native`` crate declares its own empty
    ``[workspace]``).

*External dependency*
    A dependency with no ``path`` key: a crates.io version, a ``git`` source or
    an alternate registry.  Path dependencies add no entry to the lock's
    ``source`` fields.

Which roots need a lock
-----------------------

A root needs a committed lock when any manifest in its *closure* (members plus
everything reachable through ``path`` dependencies, following ``workspace =
true`` inheritance) declares an external dependency.  A root whose closure is
path-only has nothing to pin, so it may omit the lock (``check`` does not
demand one, ``generate`` does not create one).  If such a root nevertheless has
a lock, ``check`` still verifies it is current.  The test is static (it parses
manifests with ``tomllib``) so it works before any lock exists and needs no
network.

Subcommands
-----------

``roots``
    Print every root, one per line, with ``*`` marking those needing a lock.
``generate [--root PATH]``
    ``cargo generate-lockfile`` where there is no lock; ``cargo update
    --workspace`` where there is one (it refreshes the entries for our own path
    packages but never moves a pinned third-party version).
``check [--root PATH ...] [--changed-since REF]``
    Fail if a needed lock is missing or stale.  ``--changed-since`` restricts
    the check to roots whose closure contains a changed ``Cargo.toml``,
    ``Cargo.lock`` or ``.cargo/config*``; those are the only files that can
    change what Cargo resolves.

How currency is checked (measured)
----------------------------------

Wall-clock seconds on Rust 1.99.1, Apple silicon.  "cold" means an empty
``CARGO_HOME`` (what a fresh CI runner has); the last column is how much that
run left in ``CARGO_HOME``.

===========================================  ==========  =========  =========
command                                      main ws     small, wm  small, cold
===========================================  ==========  =========  =========
``cargo update --workspace --locked``        3.2         0.27       0.45 (2.8 MB, index only)
``cargo metadata --locked``                  1.9         0.11       1.9 (23 MB, downloads crates)
``cargo metadata --locked --offline``        2.9         -          fails when sources are absent
``cargo metadata --locked --no-deps``        1.0         0.13       0.09 (never resolves: useless)
===========================================  ==========  =========  =========

(main ws = the 1,450-member workspace; small = a 35-package mosaic crate.)

``cargo metadata`` is quick when the sources are already on disk but must
download every crate's source on a cold runner, which is wasted work for a
yes/no question.  ``--no-deps`` skips resolution and so proves nothing.
``cargo update --workspace --locked`` resolves against the registry *index*
only (never a crate download, never a build), refreshes nothing it does not
have to (``--workspace`` touches only our own path packages), and exits
non-zero precisely when the lock would change, so it is both the cheapest and
a correct proof of currency.  Verified: raising a third-party requirement
(``toml = "0.8"`` to ``"0.9"``) in a manifest makes it fail.  Across all 152
locks it costs about 20 s of wall clock (warm) with four parallel jobs.

Message to authors
------------------

A stale lock fails with the exact command to run (``cargo update --workspace``
in the root directory) and the reminder to commit the result.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import glob
import os
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

DEP_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")

# The command that proves a lock is current (see "How currency is checked").
CHECK_CMD = ["cargo", "update", "--workspace", "--locked"]


# ---------------------------------------------------------------------------
# toolchain selection
# ---------------------------------------------------------------------------
#
# The lock format and resolver behaviour vary between Cargo releases, so the
# check must run the toolchain the repository pins: the ``rust`` entry of the
# ``[tools]`` table in the root ``mise.toml``.  An exact ``MAJOR.MINOR.PATCH``
# is used as ``cargo +<version>`` (installed on demand with rustup, minimal
# profile).  Anything else ("stable", "latest", ...) falls back to plain
# ``cargo``, whatever the machine provides.

EXACT_VERSION = re.compile(r"^\d+\.\d+\.\d+$")

# Set by main(); None means "plain cargo".
TOOLCHAIN: str | None = None


def pinned_rust_version(repo: Path) -> str | None:
    """The exact Rust version pinned in mise.toml, or None."""
    try:
        with open(repo / "mise.toml", "rb") as fh:
            value = tomllib.load(fh).get("tools", {}).get("rust")
    except (OSError, tomllib.TOMLDecodeError):
        return None
    if isinstance(value, str) and EXACT_VERSION.match(value):
        return value
    return None


def ensure_toolchain(version: str) -> None:
    """Install `version` with rustup (minimal profile) if it is missing."""
    listed = subprocess.run(
        ["rustup", "toolchain", "list"], capture_output=True, text=True,
    ).stdout
    if any(line.split("-")[0] == version for line in listed.splitlines()):
        return
    subprocess.run(
        ["rustup", "toolchain", "install", version, "--profile", "minimal"],
        check=True,
    )


def cargo_argv(args: list[str]) -> list[str]:
    """Prefix `cargo` with +<toolchain> when one is selected."""
    if TOOLCHAIN and args and args[0] == "cargo":
        return [args[0], f"+{TOOLCHAIN}", *args[1:]]
    return args


def repo_root() -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    return Path(out)


def tracked_manifests(repo: Path) -> list[Path]:
    """Every tracked Cargo.toml directory (absolute), sorted."""
    out = subprocess.run(
        ["git", "ls-files", "-z", "--", "*Cargo.toml"],
        cwd=repo, check=True, capture_output=True, text=True,
    ).stdout
    dirs = []
    for rel in out.split("\0"):
        if rel and Path(rel).name == "Cargo.toml":
            dirs.append((repo / rel).parent)
    return sorted(set(dirs))


# The repository's main Cargo workspace (about 1,450 member crates).
MAIN_WORKSPACE = Path("code/packages/rust")


@dataclass
class Root:
    path: Path
    closure: set[Path] = field(default_factory=set)
    needs_lock: bool = False

    @property
    def lock(self) -> Path:
        return self.path / "Cargo.lock"


class Index:
    """Parsed manifests plus the workspace/closure logic, built once."""

    def __init__(self, dirs: list[Path]):
        self.dirs = dirs
        self._toml: dict[Path, dict] = {}
        self.workspaces = [d for d in dirs if "workspace" in self.toml(d)]

    def toml(self, d: Path) -> dict:
        if d not in self._toml:
            with open(d / "Cargo.toml", "rb") as fh:
                self._toml[d] = tomllib.load(fh)
        return self._toml[d]

    # -- workspaces ------------------------------------------------------

    def ancestor_workspace(self, d: Path) -> Path | None:
        """Nearest strict ancestor directory declaring [workspace]."""
        for parent in d.parents:
            if (parent / "Cargo.toml").exists() and parent in self._ws_set():
                return parent
        return None

    def _ws_set(self) -> set[Path]:
        return set(self.workspaces)

    def explicit_members(self, ws: Path) -> set[Path]:
        data = self.toml(ws)["workspace"]
        out: set[Path] = set()
        excluded = self.excluded(ws)
        for pat in data.get("members", []):
            for hit in glob.glob(str(ws / pat)):
                p = Path(hit)
                if (p / "Cargo.toml").exists() and p not in excluded:
                    out.add(p)
        return out

    def excluded(self, ws: Path) -> set[Path]:
        data = self.toml(ws)["workspace"]
        out = set()
        for pat in data.get("exclude", []):
            out.update(Path(h) for h in glob.glob(str(ws / pat)))
        return out

    # -- dependencies ----------------------------------------------------

    def dep_specs(self, d: Path):
        """Yield (name, spec, from_dir) for the deps a manifest declares."""
        data = self.toml(d)
        tables = [data.get(t, {}) for t in DEP_TABLES]
        for tgt in data.get("target", {}).values():
            tables.extend(tgt.get(t, {}) for t in DEP_TABLES)
        for tbl in tables:
            for name, spec in tbl.items():
                yield name, spec

    def _plain(self, base: Path, spec):
        if isinstance(spec, str):
            return None, True
        if "path" in spec:
            return (base / spec["path"]).resolve(), False
        return None, True

    def inherited_dep(self, d: Path, name: str):
        ws = self.ancestor_workspace(d)
        if ws is None and d in self._ws_set():
            ws = d
        if ws is None:
            return None, True
        spec = self.toml(ws)["workspace"].get("dependencies", {}).get(name)
        if spec is None:
            return None, True
        return self._plain(ws, spec)

    def deps_of(self, d: Path):
        """(path-dep dirs, has_external) for one package directory."""
        paths: set[Path] = set()
        external = False
        for name, spec in self.dep_specs(d):
            if isinstance(spec, dict) and spec.get("workspace"):
                p, ext = self.inherited_dep(d, name)
            else:
                p, ext = self._plain(d, spec)
            if p is not None:
                paths.add(p)
            if ext:
                external = True
        data = self.toml(d)
        for sub in data.get("patch", {}).values():
            for spec in sub.values():
                p, _ = self._plain(d, spec)
                if p is not None:
                    paths.add(p)
        return paths, external

    # -- closure ---------------------------------------------------------

    def closure(self, start: set[Path]) -> tuple[set[Path], bool]:
        """Package dirs reachable from `start` via path deps; any external?"""
        seen: set[Path] = set()
        stack = list(start)
        external = False
        while stack:
            d = stack.pop()
            if d in seen:
                continue
            if not (d / "Cargo.toml").exists():
                seen.add(d)
                continue
            seen.add(d)
            paths, ext = self.deps_of(d)
            external = external or ext
            stack.extend(paths)
            ws = self.ancestor_workspace(d)
            if ws is not None and ws not in seen:
                # Inherited fields/dependencies come from this manifest too.
                seen.add(ws)
                wpaths = self.workspace_dep_paths(ws)
                stack.extend(wpaths)
        return seen, external

    def workspace_dep_paths(self, ws: Path) -> set[Path]:
        out = set()
        for spec in self.toml(ws)["workspace"].get("dependencies", {}).values():
            p, _ = self._plain(ws, spec)
            if p is not None:
                out.add(p)
        return out

    def find_roots(self) -> list[Root]:
        # 1. Fixed point: which non-workspace manifests are implicit members?
        members: set[Path] = set()
        reach_from: dict[Path, set[Path]] = {}
        for ws in self.workspaces:
            start = self.explicit_members(ws) | ({ws} if "package" in self.toml(ws) else set())
            reach_from[ws] = start
        for ws, start in reach_from.items():
            excl = self.excluded(ws)
            clo, _ = self.closure(start | {ws})
            inside = {
                p for p in clo
                if p != ws and ws in p.parents and p not in excl
                and (p / "Cargo.toml").exists()
            }
            members |= self.explicit_members(ws) | inside
        # Resolve nested workspaces: a [workspace] manifest is always a root.
        roots = []
        for d in self.dirs:
            if d in self._ws_set():
                is_root = True
            else:
                is_root = d not in members
            if is_root:
                roots.append(d)
        out = []
        for d in roots:
            start = {d}
            if d in self._ws_set():
                start |= self.explicit_members(d)
            clo, ext = self.closure(start)
            out.append(Root(d, clo, ext))
        return out


def discover(repo: Path) -> list[Root]:
    return Index(tracked_manifests(repo)).find_roots()


# ---------------------------------------------------------------------------
# changed-since
# ---------------------------------------------------------------------------

def changed_files(repo: Path, ref: str) -> list[Path] | None:
    r = subprocess.run(
        ["git", "diff", "--name-only", f"{ref}...HEAD"],
        cwd=repo, capture_output=True, text=True,
    )
    if r.returncode != 0:
        return None
    return [repo / line for line in r.stdout.splitlines() if line]


def affected(root: Root, changed: list[Path]) -> bool:
    interesting = [
        c for c in changed
        if c.name in ("Cargo.toml", "Cargo.lock")
        or (c.parent.name == ".cargo" and c.name.startswith("config"))
    ]
    for c in interesting:
        if c.parent in root.closure or c.parent == root.path:
            return True
        # .cargo/config applies to everything below its parent directory.
        if c.parent.name == ".cargo" and root.path.is_relative_to(c.parent.parent):
            return True
    return False


# ---------------------------------------------------------------------------
# commands
# ---------------------------------------------------------------------------

def run_cargo(root: Root, args: list[str]) -> tuple[int, str]:
    env = dict(os.environ, CARGO_TERM_COLOR="never")
    r = subprocess.run(
        cargo_argv(args), cwd=root.path, env=env, capture_output=True, text=True,
    )
    return r.returncode, (r.stdout + r.stderr).strip()


def select(roots: list[Root], repo: Path, wanted: list[str] | None) -> list[Root]:
    if not wanted:
        return roots
    abs_wanted = {(Path.cwd() / w).resolve() for w in wanted}
    abs_wanted |= {(repo / w).resolve() for w in wanted}
    chosen = [r for r in roots if r.path in abs_wanted]
    if not chosen:
        sys.exit(f"no Cargo root matches {wanted}")
    return chosen


def rel(repo: Path, p: Path) -> str:
    return str(p.relative_to(repo))


def select_toolchain(repo: Path) -> None:
    """Pick (and install if needed) the pinned toolchain; once per run."""
    global TOOLCHAIN
    TOOLCHAIN = pinned_rust_version(repo)
    if TOOLCHAIN:
        ensure_toolchain(TOOLCHAIN)


def cmd_roots(repo: Path, roots: list[Root], _args) -> int:
    for r in roots:
        print(("* " if r.needs_lock else "  ") + rel(repo, r.path))
    need = sum(r.needs_lock for r in roots)
    print(f"{len(roots)} roots, {need} need a lock (*)", file=sys.stderr)
    return 0


def parallel(roots, fn, jobs):
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as ex:
        yield from ex.map(fn, roots)


def cmd_generate(repo: Path, roots: list[Root], args) -> int:
    chosen = select(roots, repo, [args.root] if args.root else None)
    chosen = [r for r in chosen if r.needs_lock or r.lock.exists() or args.root]
    if chosen:
        select_toolchain(repo)

    def one(r: Root):
        argv = (["cargo", "update", "--workspace"] if r.lock.exists()
                else ["cargo", "generate-lockfile"])
        code, out = run_cargo(r, argv)
        return r, code, out

    failures = 0
    for r, code, out in parallel(chosen, one, args.jobs):
        status = "ok  " if code == 0 else "FAIL"
        print(f"{status} {rel(repo, r.path)}", flush=True)
        if code != 0:
            failures += 1
            print("     " + out.replace("\n", "\n     "), flush=True)
    return 1 if failures else 0


def check_one(repo: Path, r: Root) -> str | None:
    """Return an error message, or None when the root is fine."""
    where = rel(repo, r.path)
    if not r.lock.exists():
        if not r.needs_lock:
            return None
        return (f"{where}: Cargo.lock is missing but the project resolves "
                f"external crates.\n    fix: (cd {where} && cargo generate-lockfile) "
                f"and commit Cargo.lock")
    code, out = run_cargo(r, CHECK_CMD)
    if code == 0:
        return None
    tail = "\n    ".join(out.splitlines()[-6:])
    return (f"{where}: Cargo.lock is stale or does not resolve.\n    {tail}\n"
            f"    fix: (cd {where} && cargo update --workspace) and commit "
            f"the updated Cargo.lock")


def cmd_check(repo: Path, roots: list[Root], args) -> int:
    chosen = select(roots, repo, args.root)
    if args.changed_since:
        changed = changed_files(repo, args.changed_since)
        if changed is None:
            print(f"cannot diff against {args.changed_since}; checking all roots")
        else:
            # The main workspace is always checked, even when this branch
            # changed none of its manifests. Merging main brings in other
            # PRs' new crates, which makes the branch's copy of
            # code/packages/rust/Cargo.lock stale without any file in the
            # branch's own diff. The Release TaskApp build caught exactly
            # that (`git diff --exit-code` after cargo rewrote the lock).
            # The check costs about 3s.
            chosen = [
                r for r in chosen
                if affected(r, changed) or r.path.resolve() == (repo / MAIN_WORKSPACE).resolve()
            ]
    chosen = [r for r in chosen if r.needs_lock or r.lock.exists()]
    if chosen:
        select_toolchain(repo)
    errors = [e for e in parallel(chosen, lambda r: check_one(repo, r), args.jobs) if e]
    for e in errors:
        print(e)
    print(f"checked {len(chosen)} Cargo roots, {len(errors)} problem(s)")
    return 1 if errors else 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--jobs", type=int, default=4)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("roots")
    g = sub.add_parser("generate")
    g.add_argument("--root")
    c = sub.add_parser("check")
    c.add_argument("--root", action="append")
    c.add_argument("--changed-since", metavar="REF")
    args = ap.parse_args(argv)
    repo = repo_root()
    roots = discover(repo)
    fn = {"roots": cmd_roots, "generate": cmd_generate, "check": cmd_check}[args.cmd]
    return fn(repo, roots, args)


if __name__ == "__main__":
    sys.exit(main())
