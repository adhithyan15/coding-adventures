#!/usr/bin/env python3
"""decompose_pipeline.py — the decompose→emit→verify→explain END-TO-END pipeline (AD-5).

The closing rung of the argument-decomposer scaffold (ADJ-ARGUMENT-DECOMPOSER.md §6). It drives
a paragraph of prose through all four stages that make a decomposed argument *usable*, proving
end-to-end that **a paragraph becomes a program the engine runs, audits, and explains**:

  1. EMIT     — the paragraph's argument as an `.adj` program, every citation a verbatim byte
                slice of the paragraph (gen_argument_data.build_argument_adj).
  2. DERIVE   — `adj-lang-cli` chains the inference rules over the premise facts to DERIVE the
                paragraph's thesis (the recall answer), with no argument-specific evaluator.
  3. VERIFY   — `adj-verify --snapshots` BYTE-ANCHORS every citation against the pinned paragraph
                (each premise + each inference warrant), re-deriving the thesis.
  4. EXPLAIN  — `adj-lang-cli --explain` renders the derivation as the argument it is:
                grounded premises → the connective (inference) → the derived conclusion (ADR-6).

Stages 2–4 shell out to the built binaries; the pipeline skips them gracefully when the binaries
are absent (the EMIT stage is always pure). `test_decompose_pipeline.py` pins the four-stage flow.

Usage:
  python3 decompose_pipeline.py            # run every committed seed argument
  python3 decompose_pipeline.py --seed arg-axle-fatigue
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import gen_argument_data as gad  # noqa: E402


def _run_explain(adj_text: str) -> str:
    """Stage 4: render the argument chain via `adj-lang-cli --explain`. Returns the chain text
    (premises → connective → conclusion). Raises gad.BinariesMissing when the CLI is not built."""
    if not gad.CLI.exists():
        raise gad.BinariesMissing(f"build adj-lang-cli first: {gad.CLI}")
    with tempfile.TemporaryDirectory() as td:
        prog = Path(td) / "arg.adj"
        prog.write_text(adj_text)
        out = subprocess.run([str(gad.CLI), "--explain", str(prog)],
                             capture_output=True, text=True, timeout=60)
        return out.stdout


def run_pipeline(spec: dict) -> dict:
    """Run all four stages on one seed argument spec. Returns a report dict:
    {emitted, snapshot, derived (recall JSON text), verified, quotes_verified, explained,
     ran_cli}. When the binaries are absent, `ran_cli` is False and the CLI-stage fields are None —
     only the pure EMIT stage ran."""
    sb = gad.source_bytes_for(spec)
    adj_text, hexhash = gad.build_argument_adj(spec, sb)  # stage 1 (pure)
    report = {
        "id": spec["id"], "emitted": adj_text, "snapshot": hexhash,
        "derived": None, "verified": None, "quotes_verified": None,
        "explained": None, "ran_cli": False,
    }
    if not (gad.CLI.exists() and gad.VERIFY.exists()):
        return report
    res = gad.verify_gold(adj_text, sb)  # stages 2 (derive) + 3 (verify)
    report["ran_cli"] = True
    report["derived"] = res["derive_stdout"]
    report["verified"] = res["verified"]
    report["quotes_verified"] = res["quotes_verified"]
    report["explained"] = _run_explain(adj_text)  # stage 4
    return report


def print_report(report: dict, spec: dict) -> None:
    """A human-readable four-stage report for one seed."""
    print(f"\n=== {report['id']} ({spec.get('domain', '?')}) ===")
    print("1. EMIT — the paragraph's argument as an .adj program:")
    for line in report["emitted"].splitlines():
        print(f"     {line}")
    if not report["ran_cli"]:
        print("   (adj-lang-cli / adj-verify not built — stages 2-4 skipped)")
        return
    want = gad.total_citations(spec)
    derived_ok = spec.get("expect", "") in (report["derived"] or "")
    print(f"2. DERIVE — thesis derived: {derived_ok} (expect '{spec.get('expect', '')}')")
    print(f"3. VERIFY — byte-anchored: verified={report['verified']} "
          f"quotes_verified={report['quotes_verified']}/{want}")
    print("4. EXPLAIN — the argument chain (premises → connective → conclusion):")
    for line in (report["explained"] or "").splitlines():
        print(f"     {line}")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--seed", help="run only this seed id (default: all)")
    args = ap.parse_args()
    specs = [s for s in gad.SEED if not args.seed or s["id"] == args.seed]
    if not specs:
        print(f"decompose_pipeline: no seed matches {args.seed!r}", file=sys.stderr)
        return 2
    for spec in specs:
        print_report(run_pipeline(spec), spec)
    return 0


if __name__ == "__main__":
    sys.exit(main())
