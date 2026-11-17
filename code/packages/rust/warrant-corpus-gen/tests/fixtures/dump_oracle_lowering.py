#!/usr/bin/env python3
"""Print (or check) the independent oracle's lowering of the parity documents.

The Rust generator and the Python oracle read the same ``.warrant`` file and lower it
to the model IR separately (VER04r section 10, decision [D3]).  This script is the
oracle half of the S3a parity test: it runs the *oracle's* ``lower_document`` and
``Program`` (stdlib only, no Rust) and prints, as canonical JSON, the same
"parity view" that ``warrant_corpus_gen::view::oracle_view`` renders on the Rust side.

    python3 tests/fixtures/dump_oracle_lowering.py            # (re)write the fixtures
    python3 tests/fixtures/dump_oracle_lowering.py --check    # fail if any differs

The canonical rendering is VER04i section 9.1: keys sorted by code point, two-space
indent, LF, one final LF, non-ASCII escaped, which is exactly what
``json.dumps(tree, sort_keys=True, indent=2) + "\\n"`` produces.

Run it from anywhere; it finds the oracle package relative to its own location.  The
output files are committed next to this script so the Rust test needs no Python.  A
change to the oracle's lowering is *meant* to turn this script's ``--check`` red; it is
not wired into the crate's BUILD because that would need a declared build edge from the
Rust crate to the Python package (see the crate README).
"""

from __future__ import annotations

import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
PACKAGES = HERE.parents[3]  # code/packages
SPECS = PACKAGES.parent / "specs"
sys.path.insert(0, str(PACKAGES / "python" / "warrant-oracle" / "src"))

from coding_adventures_warrant_oracle.corpus_v1 import build_program  # noqa: E402
from coding_adventures_warrant_oracle.ir_eval import MAX_IR_COST  # noqa: E402
from coding_adventures_warrant_oracle.lower import lower_document  # noqa: E402
from coding_adventures_warrant_oracle.parser import parse_source  # noqa: E402

#: (spec file, output file, max_ir_cost).  gf256 declares ``max_ir_cost 32768`` in its
#: profile (VER04r 4.2); ct-compare declares nothing.
CASES = [
    (SPECS / "ct-compare.warrant", HERE / "ct-compare.oracle-lowering.json", MAX_IR_COST),
    (SPECS / "gf256.warrant", HERE / "gf256.oracle-lowering.json", 32768),
    (HERE / "core_forms_v0.warrant", HERE / "core_forms_v0.oracle-lowering.json", MAX_IR_COST),
    (HERE / "core_forms_v1.warrant", HERE / "core_forms_v1.oracle-lowering.json", MAX_IR_COST),
]


def _params(pairs) -> list:
    return [[name, str(ty)] for name, ty in pairs]


def view(spec_bytes: bytes, max_cost: int) -> dict:
    """The parity view of one document (see ``view.rs`` for the shape)."""
    spec = lower_document(parse_source(spec_bytes))
    for model in spec.models.values():
        if model.unsupported is not None:
            raise SystemExit(f"model {model.name} is beyond ir.core: {model.unsupported}")
    prog = build_program(spec, max_cost=max_cost)
    models = {}
    for name, m in spec.models.items():
        models[name] = {
            "params": _params(m.params),
            "ret": str(m.ret),
            "body": m.ir,
            "ir_cost": prog.model(name).cost,
        }
    suites = []
    for s in spec.suites:
        if s.unsupported is not None:
            raise SystemExit(f"claim {s.claim} is beyond ir.core: {s.unsupported}")
        tenv = dict(s.binders)
        cost = prog.compile(s.admit, tenv, allow_subject=False).cost
        cost += prog.compile(s.body, tenv, allow_subject=True).cost
        if s.kind == "reference":
            cost += prog.compile(s.expected, tenv, allow_subject=False).cost
        suites.append(
            {
                "id": s.id,
                "claim": s.claim,
                "kind": s.kind,
                "binders": _params(s.binders),
                "admit": s.admit,
                "body": s.body,
                # The canonical writer has no null: an absent side is an empty array.
                "subject": s.subject if s.kind == "reference" else [],
                "expected": s.expected if s.kind == "reference" else [],
                "result_type": str(s.result),
                "ops": list(s.ops),
                "ir_cost": cost,
            }
        )
    examples = []
    for x in spec.examples:
        if x.unsupported is not None:
            raise SystemExit(f"example {x.name} is beyond ir.core: {x.unsupported}")
        referenced = x.subject is not None
        examples.append(
            {
                "name": x.name,
                "body": x.body,
                "subject": x.subject if referenced else [],
                "expected": x.expected if referenced else [],
                "result_type": str(x.result) if referenced else "",
            }
        )
    return {
        "schema": "warrant-lowering-parity-v0",
        "module": spec.name,
        "document": spec.version,
        "errors": {name: list(cases) for name, cases in spec.errors.items()},
        "ops": {name: op.corpus_row() for name, op in spec.ops.items()},
        "model_order": list(spec.models),
        "models": models,
        "suites": suites,
        "examples": examples,
    }


def render(tree: dict) -> str:
    return json.dumps(tree, sort_keys=True, indent=2) + "\n"


def main(argv: list[str]) -> int:
    check = "--check" in argv
    bad = 0
    for spec_path, out_path, max_cost in CASES:
        text = render(view(spec_path.read_bytes(), max_cost))
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
