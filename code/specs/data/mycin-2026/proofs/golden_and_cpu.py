#!/usr/bin/env python3
"""golden_and_cpu.py - proofs: golden-rulebook (derive once, reuse at 0 calls) +
CPU-bound inference.

MYCIN-2026 M8.
  GOLDEN RULEBOOK: the rulebook was derived ONCE (the spider, cold path). Every
  case is then decided from its committed decomposition with NO model in the loop.
  We re-run the deterministic pipeline TWICE and assert (a) identical results and
  (b) answer_time_model_calls == 0 both times - the diagnosis is a pure,
  reproducible function of (decomposition, grounded rulebook).

  CPU-BOUND: time the per-case decide. Inference is a CLI call over the imported
  rulebook (parse + LR aggregation) - milliseconds, no network, no model. We
  report wall-clock per case to make "CPU-bound" concrete (not a model latency).
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
import decide as decide_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402

IR_DIR = ROOT / "ir"
CASES = ROOT / "cases" / "cases.json"


def run_once(cli, domains) -> dict:
    out = {}
    for p in sorted(IR_DIR.glob("*.json")):
        ir = json.loads(p.read_text())
        obs, _, _ = ir_mod.ir_to_adj(ir, domains)
        res = decide_mod.decide(ir["case_id"], obs, cli)
        out[ir["case_id"]] = res
    return out


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("golden_and_cpu: SKIPPED (adj-lang-cli not built)")
        return 0
    domains = ir_mod.load_domains()
    gold = {c["id"]: c["gold"] for c in json.loads(CASES.read_text())["cases"]}

    # GOLDEN RULEBOOK: derive-once, reuse; reproducible at 0 model calls.
    run_a = run_once(cli, domains)
    run_b = run_once(cli, domains)
    calls = sum(r["answer_time_model_calls"] for r in run_a.values()) + \
            sum(r["answer_time_model_calls"] for r in run_b.values())
    reproducible = all(run_a[k]["posteriors"] == run_b[k]["posteriors"] for k in run_a)
    correct = sum(1 for k, r in run_a.items()
                  if r["decision"].get("type") != "insufficient_evidence" and r["leader"] == gold[k])
    print("=== GOLDEN RULEBOOK (derive once -> reuse on every case) ===")
    print(f"  {len(run_a)} cases decided twice; identical: {reproducible}; "
          f"answer-time model calls across both runs: {calls}; "
          f"correct vs gold: {correct}/{len(run_a)}")
    assert reproducible, "non-reproducible: the rulebook is not a pure function"
    assert calls == 0, "answer-time model calls must be 0"

    # CPU-BOUND: time the per-case decide (engine over the imported rulebook).
    print("\n=== CPU-BOUND inference (wall-clock per decide; no model, no network) ===")
    timings = []
    for p in sorted(IR_DIR.glob("*.json")):
        ir = json.loads(p.read_text())
        obs, _, _ = ir_mod.ir_to_adj(ir, domains)
        t0 = time.perf_counter()
        decide_mod.decide(ir["case_id"], obs, cli)
        dt = (time.perf_counter() - t0) * 1000
        timings.append(dt)
        print(f"  {ir['case_id']:28s} {dt:7.1f} ms")
    print(f"  mean {sum(timings)/len(timings):.1f} ms/case - the reasoning is the CLI "
          f"(parse + LR aggregation), not a model.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
