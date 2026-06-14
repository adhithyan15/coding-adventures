#!/usr/bin/env python3
"""run_warm.py - the CPU-bound half of the warm path: IR -> adj -> diagnosis.

MYCIN-2026 M6. Given the decomposed IRs (ir/<id>.json, produced once by the model
in decompose.py), this runs the DETERMINISTIC pipeline over every case and scores
against the gold labels - at 0 answer-time model calls. It exists separately from
decompose.py to make the headline checkable: re-running run_warm.py reproduces
every diagnosis with no model in the loop (the golden-rulebook property).

  ir/<id>.json --ir_to_adj--> observe lines --decide--> differential + proof DAG

Writes warm/decide-results.json and prints a scored table.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
import decide as decide_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402

IR_DIR = ROOT / "ir"
CASES = ROOT / "cases" / "cases.json"
OUT = ROOT / "warm" / "decide-results.json"


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("run_warm: adj-lang-cli not built (cargo build -p adj-lang-cli)", file=sys.stderr)
        return 3
    domains = ir_mod.load_domains()
    cases = {c["id"]: c for c in json.loads(CASES.read_text())["cases"]}

    rows = []
    for ir_path in sorted(IR_DIR.glob("*.json")):
        ir = json.loads(ir_path.read_text())
        cid = ir["case_id"]
        adj, kept, dropped = ir_mod.ir_to_adj(ir, domains)
        res = decide_mod.decide(cid, adj, cli)
        gold = cases.get(cid, {}).get("gold")
        leader = res["leader"]
        dtype = res["decision"].get("type")
        # Correct iff the decisive verdict matches gold; an abstention is "abstained".
        if dtype == "insufficient_evidence":
            verdict = "abstained"
        else:
            verdict = "correct" if leader == gold else "wrong"
        rows.append({**res, "gold": gold, "kept": kept, "dropped": dropped, "score": verdict})

    n_correct = sum(1 for r in rows if r["score"] == "correct")
    n_abst = sum(1 for r in rows if r["score"] == "abstained")
    n_wrong = sum(1 for r in rows if r["score"] == "wrong")
    total_calls = sum(r["answer_time_model_calls"] for r in rows)

    summary = {
        "_doc": "Warm-path results. answer_time_model_calls_total MUST be 0 - the "
                "model only ran once per case in decompose.py; every diagnosis here is "
                "produced by the CPU engine over the imported CAS rulebook.",
        "n_cases": len(rows),
        "correct": n_correct,
        "abstained": n_abst,
        "wrong": n_wrong,
        "answer_time_model_calls_total": total_calls,
        "results": rows,
    }
    OUT.write_text(json.dumps(summary, indent=2) + "\n")

    print(f"\nwarm path: {n_correct} correct, {n_abst} abstained, {n_wrong} wrong "
          f"/ {len(rows)} cases   (answer-time model calls: {total_calls})")
    for r in rows:
        post = {k: round(v, 3) for k, v in r["posteriors"].items()}
        print(f"  {r['case_id']:28s} {r['score']:9s} leader={r['leader']} "
              f"gold={r['gold']} evidence={r['n_evidence_for_leader']} dropped={len(r['dropped'])} {post}")
    assert total_calls == 0, "answer-time model calls must be 0"
    return 0 if n_wrong == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
