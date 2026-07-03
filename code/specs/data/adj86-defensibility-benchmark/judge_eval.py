#!/usr/bin/env python3
"""ADJ86 — aggregate the BLIND judge's verdicts (un-blind A/B via judge_inputs.json).

The judge scored two unlabeled answers per (item, model) on defensibility. Here we map A/B
back to bare/framework and report, per model: mean judged defensibility (bare vs framework),
how often the judge found framework more defensible, and the underdetermined-subclass split.

Usage: python judge_eval.py <judge-results.json>
"""
from __future__ import annotations

import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
INPUTS = json.load(open(os.path.join(HERE, "judge_inputs.json")))


def main():
    res = json.loads(open(sys.argv[1]).read())
    res = res.get("result", res)
    verdicts = res["verdicts"] if "verdicts" in res else res
    if len(verdicts) != len(INPUTS):
        print(f"!! count mismatch judge={len(verdicts)} inputs={len(INPUTS)} — index alignment unreliable")
        sys.exit(2)

    cells = defaultdict(lambda: {"n": 0, "bare_sum": 0.0, "fw_sum": 0.0, "fw_wins": 0, "ties": 0,
                                 "ud_n": 0, "ud_fw_wins": 0, "ud_bare_sum": 0.0, "ud_fw_sum": 0.0})
    detail = []
    for e, v in zip(INPUTS, verdicts):  # same order (parallel preserves input order)
        bare_is = e["bare_is"]
        bare_s = v["A_score"] if bare_is == "A" else v["B_score"]
        fw_s = v["B_score"] if bare_is == "A" else v["A_score"]
        bare_label, fw_label = ("A", "B") if bare_is == "A" else ("B", "A")
        fw_more = v["more_defensible"] == fw_label
        tie = v["more_defensible"] == "tie"
        c = cells[e["model"]]
        c["n"] += 1
        c["bare_sum"] += bare_s
        c["fw_sum"] += fw_s
        c["fw_wins"] += fw_more
        c["ties"] += tie
        if e["stratum"] == "underdetermined-baited":
            c["ud_n"] += 1
            c["ud_fw_wins"] += fw_more
            c["ud_bare_sum"] += bare_s
            c["ud_fw_sum"] += fw_s
        detail.append({"id": e["id"], "model": e["model"], "stratum": e["stratum"],
                       "bare_score": bare_s, "fw_score": fw_s, "more_defensible": "framework" if fw_more else ("tie" if tie else "bare"),
                       "fw_verdict": e["fw_verdict"]})

    print("=" * 80)
    print("  ADJ86 — BLIND judge: defensibility of bare vs framework (un-blinded)")
    print("=" * 80)
    print(f"  {'model':8} {'bare def (judge)':>18} {'framework def':>16} {'fw more-defensible':>20}")
    print("-" * 80)
    for m in ("haiku", "opus"):
        c = cells.get(m)
        if not c:
            continue
        n = c["n"]
        more = f"{c['fw_wins']}/{n} ({c['ties']} ties)"
        print(f"  {m:8} {c['bare_sum'] / n:18.2f} {c['fw_sum'] / n:16.2f} {more:>20}")
    print("\n  Underdetermined-baited subclass (where weak-model fabrication shows):")
    for m in ("haiku", "opus"):
        c = cells.get(m)
        if not c or not c["ud_n"]:
            continue
        u = c["ud_n"]
        print(f"    {m:8}  bare {c['ud_bare_sum']/u:.2f}  vs  framework {c['ud_fw_sum']/u:.2f}   (framework more-defensible {c['ud_fw_wins']}/{u})")
    print("\n  HEADLINE — does the judge rate framework-Haiku as defensible as Opus?")
    h, o = cells.get("haiku", {}), cells.get("opus", {})
    if h and o:
        print(f"    judged defensibility:  bare-Haiku {h['bare_sum']/h['n']:.2f}  ->  FW-Haiku {h['fw_sum']/h['n']:.2f}   |   bare-Opus {o['bare_sum']/o['n']:.2f}  FW-Opus {o['fw_sum']/o['n']:.2f}")
    json.dump({"detail": detail, "cells": {k: dict(v) for k, v in cells.items()}},
              open(os.path.join(HERE, "judge_eval_results.json"), "w"), indent=2)


if __name__ == "__main__":
    main()
