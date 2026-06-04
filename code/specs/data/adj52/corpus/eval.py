#!/usr/bin/env python3
"""Generic, domain-agnostic case evaluator — adjudicates one case against any
grounded corpus as a deterministic single-hypothesis sequential Bayesian update.

    logodds(target) = logit(prior_prevalence)
    for each observed finding the corpus has a node for:
        logodds += ln(LR_finding)
    P(target) = sigmoid(logodds)

`grounded` mode trusts only LRs the spider graded `grounded`; ungrounded findings
are reported as data-gaps that did NOT move the posterior (honest abstention, no
invented push). `all` mode uses every LR (for contrast).

Case JSON: {"name":..., "ground_truth":..., "observations":["finding(state)", ...]}.

Run: python eval.py <corpus.json> <case.json> [grounded|all]
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path


def sigmoid(x: float) -> float:
    return 1.0 / (1.0 + math.exp(-x))


def main() -> None:
    corpus = json.loads(Path(sys.argv[1]).read_text())
    case = json.loads(Path(sys.argv[2]).read_text())
    mode = sys.argv[3] if len(sys.argv) > 3 else "grounded"

    nodes = {f"{n['finding']}({n['state']})": n for n in corpus["findings"]}
    prior = corpus["prior"]
    p0 = prior["lr"]
    logodds = math.log(p0 / (1 - p0))
    target = corpus["target"]

    print(f"=== {corpus['domain']}: {case.get('name','?')} — mode: {mode} ===")
    print(f"target: {target}")
    print(f"ground truth: {case.get('ground_truth','?')}\n")
    print(f"  prior P = {p0:.3f}   [{'GROUNDED' if prior['grounded'] else 'ungrounded'}] {prior['provenance']['study'][:52]}")

    gaps = []
    for obs in case["observations"]:
        n = nodes.get(obs)
        if n is None:
            gaps.append((obs, "not in corpus (out of scope)"))
            continue
        if mode == "grounded" and not n["grounded"]:
            gaps.append((obs, f"ungrounded ({n['verdict']}) — did not move posterior"))
            continue
        lr = n["lr"]
        if not lr or lr <= 0:
            gaps.append((obs, "no usable LR"))
            continue
        logodds += math.log(lr)
        print(f"  x LR {lr:<6} ({obs:44s}) -> P = {sigmoid(logodds):.3f}   "
              f"[{'grounded' if n['grounded'] else n['verdict']}] {n['provenance']['study'][:30]}")

    print(f"\n  >>> P({target}) = {sigmoid(logodds):.4f}")
    if gaps:
        print("\n  Data-gaps (did NOT move the posterior — honest abstention, no invented LR):")
        for obs, why in gaps:
            print(f"    - {obs}: {why}")


if __name__ == "__main__":
    main()
