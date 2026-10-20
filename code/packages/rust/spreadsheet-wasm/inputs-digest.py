#!/usr/bin/env python3
"""inputs-digest.py — the SHA-256 of everything `pkg/spreadsheet_engine.wasm`
is built from.

Why this exists
---------------
`pkg/spreadsheet_engine.wasm` is a committed build artifact: the VisiCalc web
demos embed it and are opened straight from disk, so there is no build step
to produce it on demand. A committed artifact goes stale silently. This one
did: it was last rebuilt for #7713, so later spreadsheet-core fixes (#15541,
"calculate edited formulas before dependents", among others) never reached the
demos.

Comparing the committed bytes with a fresh build would catch that, but only if
two machines produce identical bytes, which depends on toolchains and paths.
The *inputs* are the same on every machine. So `build-wasm.sh` records this
digest beside the artifact, and `build-wasm.sh --check` recomputes it: any
change to a source the engine is built from, or to a dependency's version,
fails the check until the artifact is rebuilt.

What counts as an input
-----------------------
`cargo tree` for `spreadsheet-wasm` on the wasm target (normal and build
edges) lists every crate in the build. For each crate, its name and version go
into the digest. For a path crate (one in this repository), so do its
`Cargo.toml`, its `build.rs` if any, and every file under `src/`, in sorted
order, each prefixed by its path relative to the crate, so that a rename
changes the digest too.

    +-----------------------+      +------------------------+
    | cargo tree --format   | ---> | name version (path)    |  one line per crate
    +-----------------------+      +------------------------+
                                              |
                                              v
                          sha256( "name version\n" + files... ) per crate,
                          crates in (name, version) order

Usage:  cargo tree ... --format "{p}" | python3 inputs-digest.py
"""

import hashlib
import os
import re
import sys

# `{p}` renders as `name vX.Y.Z` or `name vX.Y.Z (/abs/path)`, optionally
# followed by markers such as `(*)` (already shown above) or `(proc-macro)`.
LINE = re.compile(
    r"^(?P<name>\S+) (?P<version>v\S+)(?: \((?P<path>/[^)]*|[A-Za-z]:\\[^)]*)\))?"
)


def crate_files(path):
    """The files of a path crate that feed its build, sorted, `/`-separated."""
    files = [name for name in ("Cargo.toml", "build.rs") if os.path.isfile(os.path.join(path, name))]
    for root, _dirs, names in os.walk(os.path.join(path, "src")):
        files.extend(os.path.relpath(os.path.join(root, name), path) for name in names)
    return sorted(rel.replace(os.sep, "/") for rel in files)


def main():
    crates = set()
    for raw in sys.stdin:
        match = LINE.match(raw.strip())
        if match:
            crates.add((match["name"], match["version"], match["path"]))
    if not crates:
        sys.exit("inputs-digest.py: cargo tree listed no crates")
    digest = hashlib.sha256()
    for name, version, path in sorted(crates, key=lambda crate: (crate[0], crate[1])):
        digest.update(f"{name} {version}\n".encode())
        if path:
            for rel in crate_files(path):
                digest.update(rel.encode() + b"\0")
                with open(os.path.join(path, rel), "rb") as handle:
                    digest.update(handle.read())
                digest.update(b"\0")
    print(digest.hexdigest())


if __name__ == "__main__":
    main()
