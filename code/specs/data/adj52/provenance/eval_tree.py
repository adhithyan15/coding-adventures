#!/usr/bin/env python3
"""Evaluate the case-5 query DIRECTLY on the tree-shaped JSON rulebook — no
adj-lang `program`, no engine compile. Experiment B.

Posterior for each hypothesis is computed from the tree:
    logodds = ln(prior / (1 - prior))
    for each ACTIVE evidence node (a finding that is observed, or a mechanism
    with >=1 observed manifestation -> fires ONCE):
        logodds += ln(effective_lr(node, mode))
    posterior = sigmoid(logodds)

The tree makes correlation structure explicit: a mechanism's manifestations are
children of one node, so they contribute their combined LR once by construction —
the over-stacking the flat program permits cannot happen here.

MODES (the experiment):
  as_derived          - use every LR as the deriver wrote it (reproduces the engine).
  grounded_only       - trust ONLY magnitudes the provenance spider graded `grounded`;
                        every other LR -> 1.0 (no contribution). "What survives if we
                        only believe numbers that trace to data."
  direction_preserving- keep the DIRECTION of ungrounded LRs but strip the invented
                        magnitude: raise -> 2.0, lower -> 0.5. "Keep the clinical
                        direction, drop the false precision."

Run: python eval_tree.py [as_derived|grounded_only|direction_preserving]
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TREE = json.loads((HERE / "case5-tree.json").read_text())


def sigmoid(x: float) -> float:
    return 1.0 / (1.0 + math.exp(-x))


def effective_lr(node: dict, mode: str) -> float:
    lr = node["lr"]
    if mode == "as_derived":
        return lr
    grounded = node.get("provenance_verdict") == "grounded"
    if grounded:
        return lr
    if mode == "grounded_only":
        return 1.0
    if mode == "direction_preserving":
        return 2.0 if lr > 1.0 else (0.5 if lr < 1.0 else 1.0)
    raise ValueError(mode)


def is_active(node: dict) -> bool:
    return node["observed"] if node["type"] == "finding" else node["fires"]


def evaluate(mode: str) -> list[dict]:
    out = []
    for h in TREE["hypotheses"]:
        p = h["prior"]
        logodds = math.log(p / (1 - p))
        fired = []
        for node in h["evidence"]:
            if not is_active(node):
                continue
            lr = effective_lr(node, mode)
            if lr == 1.0:
                continue
            logodds += math.log(lr)
            tag = node.get("name", node.get("finding", ""))
            fired.append((tag, node["lr"], lr, node.get("provenance_verdict", "unknown")))
        out.append({"dx": h["dx"], "posterior": sigmoid(logodds), "fired": fired})
    out.sort(key=lambda r: -r["posterior"])
    return out


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else "as_derived"
    ranked = evaluate(mode)
    print(f"=== case-5 tree evaluation — mode: {mode} ===")
    print(f"ground truth: {TREE['ground_truth']}\n")
    for i, r in enumerate(ranked):
        star = "  <-- TRUE DX" if "carcinoma" in r["dx"] else ""
        print(f"  {i + 1}. {r['dx']:50s} P = {r['posterior']:.4f}{star}")
    top = ranked[0]
    print(f"\n  top-1: {top['dx']}  @ {top['posterior']:.4f}")
    if mode != "as_derived":
        print("\n  fired evidence on top-1 (orig_lr -> effective_lr [verdict]):")
        for tag, orig, eff, verdict in top["fired"]:
            mark = "" if orig == eff else "  *adjusted*"
            print(f"    {tag:50s} {orig} -> {eff}  [{verdict}]{mark}")


if __name__ == "__main__":
    main()
