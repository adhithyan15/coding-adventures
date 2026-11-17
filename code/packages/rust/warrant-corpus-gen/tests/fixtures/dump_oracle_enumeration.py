#!/usr/bin/env python3
"""Print (or check) the independent oracle's enumeration of the S3b parity documents.

S3b of the generator (``warrant_corpus_gen::enumerate``) and the Python oracle
(``python/warrant-oracle``, ``emit``) enumerate the same ``.warrant`` file separately:
product order, admission, chunks, counts, ``wraps``, the 18 v1 digests of ct-compare and
every chunk line (VER04r section 10.3, decision [D3]).  This script is the oracle half
of ``tests/enumeration_parity.rs``: it runs the oracle's ``emit`` at ``full`` depth
(stdlib only, no Rust) and prints, as canonical JSON, the *parity view* that
``warrant_corpus_gen::enumerate::enumeration_view`` renders on the Rust side.

    python3 tests/fixtures/dump_oracle_enumeration.py            # (re)write the snapshots
    python3 tests/fixtures/dump_oracle_enumeration.py --check    # fail if any differs

Canonical rendering is VER04i section 9.1 (``json.dumps(tree, sort_keys=True, indent=2)``
plus a final LF).  The snapshots are committed next to this script so the Rust test needs
no Python; the oracle package's BUILD runs ``--check`` so a change to the oracle's
enumeration turns CI red here, not silently in the generator.

The oracle's ``slim_chunks`` is the base set only; witness chunks are S3c/S4 part 2.
"""

from __future__ import annotations

import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
PACKAGES = HERE.parents[3]  # code/packages
SPECS = PACKAGES.parent / "specs"
sys.path.insert(0, str(PACKAGES / "python" / "warrant-oracle" / "src"))

from coding_adventures_warrant_oracle.emit import emit  # noqa: E402

#: (spec file, snapshot file, chunk_size).  ct-compare uses the default 65,536; the
#: synthetic documents use 1,024 so that the 65,536-ordinal suites have 64 chunks and the
#: slim stride (m > 16) is exercised.
CASES = [
    (SPECS / "ct-compare.warrant", HERE / "ct-compare.oracle-enumeration.json", 1 << 16),
    (HERE / "enum_forms_v1.warrant", HERE / "enum_forms_v1.oracle-enumeration.json", 1 << 10),
    (HERE / "enum_forms_v0.warrant", HERE / "enum_forms_v0.oracle-enumeration.json", 1 << 16),
    # 13 suites over concat/take/drop, and/or/implies/not, ordering comparisons in `expected`,
    # exact Nat/Int arithmetic, wraps at U16/U32/U64, Bytes literals, a model call in `admit`
    # and wrapping pure guards under or / =>  (from the S3b review)
    (HERE / "enum_probe.warrant", HERE / "enum_probe.oracle-enumeration.json", 1 << 10),
]

#: row key in the oracle's ``emit`` -> key in the parity view (the others are dropped).
KEYS = [
    ("id", "id"),
    ("claim", "claim"),
    ("kind", "kind"),
    ("binders", "binders"),
    ("descriptors", "descriptors"),
    ("demoted", "demoted"),
    ("domain_label", "domain_label"),
    ("stream_header", "stream_header"),
    ("result_type", "result_type"),
    ("product", "product"),
    ("chunk_size", "chunk_size"),
    ("chunks_total", "chunks_total"),
    ("cases", "cases"),
    ("discarded", "discarded"),
    ("expected_calls", "expected_calls"),
    ("wraps", "wraps"),
    ("ir_cost", "ir_cost"),
    ("slim_chunks", "slim_chunks"),
    ("slim_inputs_sha256", "slim_inputs_sha256"),
    ("slim_expected_sha256", "slim_expected_sha256"),
    ("inputs_sha256", "inputs_sha256"),
    ("expected_sha256", "expected_sha256"),
    ("v1_inputs_sha256", "v1_inputs_sha256"),
    ("v1_expected_sha256", "v1_expected_sha256"),
    ("chunk_lines", "chunk_lines"),
]


def view(spec_bytes: bytes, chunk_size: int) -> dict:
    """The parity view of one document (see ``enumerate.rs`` for the shape)."""
    tree = emit(spec_bytes, depth="full", chunk_size=chunk_size)
    suites = {}
    for claim, row in tree["suites"].items():
        if row["status"] != "ok":
            raise SystemExit(f"claim {claim} is not enumerable: {row}")
        # The canonical writer has no null: an absent digest is an absent key.
        suites[claim] = {new: row[old] for old, new in KEYS if row.get(old) is not None}
    return {
        "schema": "warrant-enumeration-parity-v0",
        "module": tree["module"],
        "document": tree["document"],
        "chunk_size": tree["chunk_size"],
        "budget": tree["budget"],
        "max_ir_cost": tree["max_ir_cost"],
        "suites": suites,
    }


def render(tree: dict) -> str:
    return json.dumps(tree, sort_keys=True, indent=2) + "\n"


def main(argv: list[str]) -> int:
    check = "--check" in argv
    bad = 0
    for spec_path, out_path, chunk_size in CASES:
        text = render(view(spec_path.read_bytes(), chunk_size))
        if check:
            if not out_path.exists() or out_path.read_bytes() != text.encode("utf-8"):
                print(f"DIFFERS: {out_path.name}", file=sys.stderr)
                bad += 1
            else:
                print(f"ok: {out_path.name}")
        else:
            out_path.write_bytes(text.encode("utf-8"))
            print(f"wrote {out_path.name} ({len(text)} bytes)")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
