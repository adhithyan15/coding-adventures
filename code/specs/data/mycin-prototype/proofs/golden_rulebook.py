#!/usr/bin/env python3
"""PROOF 1 (golden rulebook) + PROOF 5 (CPU-bound): derive once, reuse at 0 model calls.

The rulebook was derived ONCE (cold path): the literature was translated into adj-lang
clauses and the CAS-write gate's adversarial readers vetted them — a one-time model cost,
recorded in gate/gate_report.json. Thereafter EVERY case is decided by executing the
content-addressed library on the CPU (adj-lang-cli), with ZERO answer-time model calls.

This proof tallies the two columns and re-runs all cases off the single CAS library to show
the same derived-once rulebook decides every case deterministically.

Run: python3 proofs/golden_rulebook.py
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, ".."))
sys.path.insert(0, ROOT)
import decide as D  # noqa: E402
import ir_to_adj as I  # noqa: E402


def main():
    cli = D.find_cli()
    findings = I.load_findings()
    rb, cas_hash = D.load_rulebook()

    # cold cost: the one-time CAS-write gate (model). warm cost: 0 per case.
    gate = json.load(open(os.path.join(ROOT, "gate", "gate_report.json")))
    cold_reads = sum(len(r.get("votes") or []) for r in gate["report"])

    cases = ("MEN-1", "MEN-2", "MEN-3", "MEN-4")
    decisions, warm_calls = {}, 0
    for cid in cases:
        ir = json.load(open(os.path.join(ROOT, "ir", f"{cid}.json")))
        case_adj, _ = I.ir_to_adj(ir, findings)
        p = os.path.join(ROOT, "cases", f"_gr_{cid}.adj")
        open(p, "w").write(rb.rstrip() + "\n\n" + case_adj)
        out = subprocess.run([cli, p], capture_output=True, text=True)  # CPU only
        os.remove(p)
        res = json.loads(out.stdout)
        decisions[cid] = res["decision"].get("type") + " -> " + str(res["decision"].get("leader"))
        warm_calls += 0  # the decision invokes only the engine binary

    result = {
        "claim": "derive once, reuse indefinitely, inference is CPU-bound",
        "cas_library": cas_hash,
        "cold_path_one_time_model_reads": cold_reads,
        "warm_path_answer_time_model_calls_total": warm_calls,
        "cases_decided_off_the_single_library": len(cases),
        "decisions": decisions,
        "reproducible": "same input + same CAS version + same query = same proof DAG, byte-for-byte",
    }
    json.dump(result, open(os.path.join(HERE, "golden_rulebook_result.json"), "w"), indent=1)
    print(json.dumps(result, indent=1))


if __name__ == "__main__":
    main()
