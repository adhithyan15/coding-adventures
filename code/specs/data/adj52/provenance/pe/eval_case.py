#!/usr/bin/env python3
"""Phase 3: adjudicate one PE case AGAINST the frozen grounded corpus.

PE is a single-hypothesis sequential Bayesian update:
    logodds(PE) = logit(prior_prevalence)
    for each observed finding that the corpus has a node for:
        logodds += ln(LR_finding)
    P(PE) = sigmoid(logodds)

Two modes:
  grounded  - use ONLY corpus LRs the spider graded `grounded` (the auditable
              corpus). Ungrounded findings are reported as data-gaps that did NOT
              move the posterior — honest abstention rather than an invented push.
  all       - use every corpus LR regardless of grounding (for contrast).

A case file is JSON: {"name":..., "ground_truth":..., "observations":["d_dimer(elevated)", ...]}.
Each observation must match a corpus node's finding(state).

Run: python eval_case.py <case.json> [grounded|all]
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
# Read the canonical grounded corpus from its first-class home under corpus/.
CORPUS_PATH = HERE.parent.parent / "corpus" / "pulmonary_embolism" / "corpus.json"
CORPUS = json.loads(CORPUS_PATH.read_text())


def sigmoid(x: float) -> float:
    return 1.0 / (1.0 + math.exp(-x))


def node_for(obs: str):
    for n in CORPUS["findings"]:
        if f"{n['finding']}({n['state']})" == obs:
            return n
    return None


def evaluate(case: dict, mode: str) -> dict:
    prior = CORPUS["prior"]
    p0 = prior["lr"]
    logodds = math.log(p0 / (1 - p0))
    steps = [{"step": "prior", "value": p0, "logodds": logodds,
              "prov": prior["provenance"], "grounded": prior["grounded"]}]
    gaps = []
    for obs in case["observations"]:
        n = node_for(obs)
        if n is None:
            gaps.append({"obs": obs, "reason": "not in corpus (out of scope)"})
            continue
        if mode == "grounded" and not n["grounded"]:
            gaps.append({"obs": obs, "reason": f"ungrounded ({n['verdict']}) — did not move posterior"})
            continue
        lr = n["lr"]
        if not lr or lr <= 0:
            gaps.append({"obs": obs, "reason": "no usable LR"})
            continue
        logodds += math.log(lr)
        steps.append({"step": obs, "lr": lr, "logodds": logodds,
                      "p_running": sigmoid(logodds), "prov": n["provenance"],
                      "grounded": n["grounded"]})
    return {"posterior": sigmoid(logodds), "steps": steps, "gaps": gaps}


def main() -> None:
    case = json.loads(Path(sys.argv[1]).read_text())
    mode = sys.argv[2] if len(sys.argv) > 2 else "grounded"
    r = evaluate(case, mode)
    print(f"=== PE case: {case.get('name','?')} — mode: {mode} ===")
    print(f"ground truth: {case.get('ground_truth','?')}\n")
    print("Sequential Bayesian update (every step traces to a study):")
    for s in r["steps"]:
        if s["step"] == "prior":
            print(f"  prior P(PE) = {s['value']:.3f}   [{ 'GROUNDED' if s['grounded'] else 'ungrounded'}] "
                  f"{s['prov']['study'][:50]}")
        else:
            print(f"  x LR {s['lr']:<5} ({s['step']:42s}) -> P = {s['p_running']:.3f}   "
                  f"[{'grounded' if s['grounded'] else s.get('verdict','?')}] {s['prov']['study'][:34]}")
    print(f"\n  >>> P(PE) = {r['posterior']:.4f}")
    if r["gaps"]:
        print("\n  Data-gaps (did NOT move the posterior — honest abstention, no invented LR):")
        for g in r["gaps"]:
            print(f"    - {g['obs']}: {g['reason']}")


if __name__ == "__main__":
    main()
