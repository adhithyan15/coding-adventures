#!/usr/bin/env python3
"""Write or check the ``v2-*.json`` categories of ``warrant-corpus-conformance-v1`` (VER04r section 11, S5a, S5c).

The kit's case tables are hand-written beside this script (``cases_*.py``) and state each case's inputs and
its *intent*; ``coding_adventures_warrant_oracle.conformance`` derives every outcome independently, and the
build fails naming the case id if the two disagree.  **No fixture JSON is edited by hand.**

    make_conformance.py --write [--fixture DIR]   write the seven v2-*.json files into DIR
    make_conformance.py --check [--fixture DIR]   rebuild them in memory and require DIR's bytes to be identical

``DIR`` defaults to the family directory ``code/specs/fixtures/warrant-corpus-conformance-v1``.  This
script **never writes** ``index.json`` (the family's v1 script does, from its four derived categories plus
one row per ``v2-*.json`` on disk, section 11 R26); ``--check`` also requires an ``index.json`` that is
present to hold exactly one row per file derived here, with that file's ``cases`` and ``sha256``, and skips
that one check in a directory that has no ``index.json`` (S5a's scratch run).

Exit codes (the oracle's convention, R19): ``0`` identical (or written), ``1`` the committed bytes differ,
``2`` a table and its derivation disagree, or any other error.

**``v2-binding.json`` (S5c, committed by S5d) is checked like every other category.**  A missing file, a missing
index row, or both together, is exit 1: the "not committed yet" branch that S5c left for the six-file directory is
removed (VER04r section 16.19 R93), so removing the file and its row can never pass.

The two kit documents are derived once per run: ``kit.warrant`` takes about a minute (it holds a ``2**24``
suite, 256 chunks), ``kit-small.warrant`` a fraction of a second.
"""

from __future__ import annotations

import argparse
import functools
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from coding_adventures_warrant_oracle import bindings_v2 as BV  # noqa: E402
from coding_adventures_warrant_oracle import conformance as C  # noqa: E402

#: ``code/specs/fixtures/warrant-corpus-conformance-v1``: HERE is ``code/packages/python/warrant-oracle/tests/fixtures/conformance-v2``.
FAMILY = HERE.parents[5] / "specs" / "fixtures" / "warrant-corpus-conformance-v1"

#: The kit documents: name, ``.warrant`` file, ``corpus.profile`` file.
KITS = (
    ("kit", "kit.warrant", "kit.profile"),
    ("small", "kit-small.warrant", "kit-small.profile"),
    ("width", "kit-width.warrant", "kit-width.profile"),
    ("tuple", "kit-tuple.warrant", "kit-tuple.profile"),
)


@functools.cache
def load_kits() -> dict:
    """Derive both kit documents (the expensive part of a build), once per process."""
    return {
        name: C.build_kit((HERE / warrant).read_bytes(), (HERE / profile).read_bytes(), spec_ref=f"conformance-v2/{warrant}")
        for name, warrant, profile in KITS
    }


def load_tables(kits: dict) -> dict:
    """The six case tables, as ``{category: [Row, ..]}`` in table order."""
    import cases_arith
    import cases_chunk
    import cases_domain
    import cases_fault
    import cases_ir
    import cases_value

    return {
        "v2-value": cases_value.ROWS,
        "v2-arith": cases_arith.ROWS,
        "v2-ir": cases_ir.ROWS,
        "v2-domain": cases_domain.ROWS,
        "v2-chunk": cases_chunk.build_rows(kits),
        "v2-fault": cases_fault.ROWS,
    }


def build() -> tuple[dict, list]:
    """``({file name: text}, [index rows])`` of the six S5a categories, each case checked against its table intent."""
    kits = load_kits()
    return C.build_files(load_tables(kits), kits)


#: The seventh category's file (S5c).
BINDING_FILE = f"{BV.CATEGORY}.json"


def build_binding() -> tuple[str, dict]:
    """``(text of v2-binding.json, its index row)``: the validator derives every case of ``cases_binding.py``."""
    import cases_binding

    return BV.build_binding(cases_binding.ROWS)


def write(directory: pathlib.Path) -> int:
    files, rows = build()
    text, binding_row = build_binding()
    files, rows = {**files, BINDING_FILE: text}, [*rows, binding_row]
    directory.mkdir(parents=True, exist_ok=True)
    for name, text in files.items():
        (directory / name).write_bytes(text.encode("ascii"))  # bytes, never text mode: LF stays LF
    for row in rows:
        print(f"{row['path']}: {row['cases']} cases")
    print(f"total: {sum(r['cases'] for r in rows)} cases in {len(rows)} files -> {directory}")
    return 0


def check(directory: pathlib.Path) -> int:
    files, rows = build()
    text, binding_row = build_binding()
    stale = []
    index = directory / "index.json"
    held = None
    if index.is_file():
        try:
            held = [r for r in json.loads(index.read_bytes())["files"] if str(r.get("category", "")).startswith("v2-")]
        except (ValueError, KeyError, TypeError):
            held = None
    files, rows = {**files, BINDING_FILE: text}, [*rows, binding_row]
    for name, derived in files.items():
        path = directory / name
        if not path.is_file() or path.read_bytes() != derived.encode("ascii"):
            stale.append(f"{name} differs from the derived bytes")
    if index.is_file() and held != rows:
        stale.append("index.json does not hold exactly one row per derived file, with its cases and sha256")
    for message in stale:
        print(f"make_conformance: {message}", file=sys.stderr)
    if not stale:
        print(f"make_conformance: {sum(r['cases'] for r in rows)} cases in {len(rows)} files are identical")
    return 1 if stale else 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="write the seven v2-*.json files")
    mode.add_argument("--check", action="store_true", help="require the bytes on disk to be identical")
    parser.add_argument("--fixture", type=pathlib.Path, default=FAMILY, help="the family directory")
    args = parser.parse_args(argv)
    try:
        return write(args.fixture) if args.write else check(args.fixture)
    except C.KitError as exc:
        print(f"make_conformance: a table and its derivation disagree: {exc}", file=sys.stderr)
        return 2
    except Exception as exc:  # any other error is exit 2 too (R19)
        print(f"make_conformance: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
