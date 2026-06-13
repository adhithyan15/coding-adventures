#!/usr/bin/env python3
"""ADJ99 defensibility rescore — aggregation.

Joins the counterfactual-rubric verdicts (verdicts.json, produced by
rejudge.workflow.js) back to arm / gold / old-score via cell_map.json, and
produces the corrected headline table plus the old-vs-new comparison.

The question this answers: once defensibility is scored as LOCUS-EXPOSURE
(is the load-bearing premise named and flagged fallible?) instead of citation
density, AND the format is normalized so the judge cannot read the arm off the
style — does the fw advantage survive?

Run:  python3 rescore_aggregate.py
"""
import json
import os
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ARMS = ["plain-haiku", "plain-opus", "fw-haiku", "fw-opus"]

cell_map = {c["idx"]: c for c in json.load(open(os.path.join(HERE, "cell_map.json")))}
verdicts = json.load(open(os.path.join(HERE, "verdicts.json")))
vby = {v["idx"]: v for v in verdicts}


def mean(xs):
    xs = [x for x in xs if x is not None]
    return round(sum(xs) / len(xs), 3) if xs else None


def is_correct(acc):
    return acc == "correct"


rows_by_arm = defaultdict(list)
for idx, v in vby.items():
    c = cell_map[idx]
    rows_by_arm[c["arm"]].append({
        "idx": idx,
        "arm": c["arm"],
        "old_def": c["old_def"],
        "new_def": v["defensibility"],
        "correct": is_correct(c["old_acc"]),
        "premise_named": v["premise_named_by_solution"],
        "premise_flagged_fallible": v["premise_flagged_fallible"],
        "would_flip": v["states_what_would_flip_answer"],
    })

table = {}
for arm in ARMS:
    rows = rows_by_arm[arm]
    n = len(rows)
    table[arm] = {
        "n_scored": n,
        "old_mean_def": mean([r["old_def"] for r in rows]),
        "new_mean_def": mean([r["new_def"] for r in rows]),
        "old_def_ge4": sum(1 for r in rows if r["old_def"] is not None and r["old_def"] >= 4),
        "new_def_ge4": sum(1 for r in rows if r["new_def"] >= 4),
        "pct_premise_named": round(sum(r["premise_named"] for r in rows) / n, 3),
        "pct_premise_flagged_fallible": round(sum(r["premise_flagged_fallible"] for r in rows) / n, 3),
        "pct_would_flip": round(sum(r["would_flip"] for r in rows) / n, 3),
        # new-def vs correctness (defensibility should be ~independent of correctness)
        "new_mean_def_correct": mean([r["new_def"] for r in rows if r["correct"]]),
        "new_mean_def_incorrect": mean([r["new_def"] for r in rows if not r["correct"]]),
        "n_correct": sum(1 for r in rows if r["correct"]),
    }

# Cross-cutting: how often does new def>=4 land on a WRONG answer (old rubric: 70.3%)?
allrows = [r for rs in rows_by_arm.values() for r in rs]
new_ge4 = [r for r in allrows if r["new_def"] >= 4]
new_ge4_wrong = [r for r in new_ge4 if not r["correct"]]

# Headline deltas (the confound test): does the fw>plain gap shrink under
# format-normalized, construct-valid scoring?
def gap(metric):
    return {
        "haiku_fw_minus_plain": round(table["fw-haiku"][metric] - table["plain-haiku"][metric], 3),
        "opus_fw_minus_plain": round(table["fw-opus"][metric] - table["plain-opus"][metric], 3),
    }

summary = {
    "per_arm": table,
    "fw_minus_plain_gap": {
        "old_mean_def": gap("old_mean_def"),
        "new_mean_def": gap("new_mean_def"),
    },
    "new_def_ge4_total": len(new_ge4),
    "new_def_ge4_wrong": len(new_ge4_wrong),
    "new_def_ge4_wrong_fraction": round(len(new_ge4_wrong) / len(new_ge4), 3) if new_ge4 else None,
    "n_verdicts": len(verdicts),
}

with open(os.path.join(HERE, "rescore_summary.json"), "w") as f:
    json.dump(summary, f, ensure_ascii=False, indent=2)

# Pretty headline table
print("\n=== CORRECTED DEFENSIBILITY (counterfactual rubric, format-normalized) ===\n")
hdr = "{:<13} {:>6} {:>8} {:>8} {:>9} {:>9} {:>9} {:>9}".format(
    "arm", "n", "old_def", "new_def", "named%", "flag%", "flip%", "new≥4")
print(hdr)
print("-" * len(hdr))
for arm in ARMS:
    t = table[arm]
    print("{:<13} {:>6} {:>8} {:>8} {:>9} {:>9} {:>9} {:>9}".format(
        arm, t["n_scored"], t["old_mean_def"], t["new_mean_def"],
        t["pct_premise_named"], t["pct_premise_flagged_fallible"],
        t["pct_would_flip"], t["new_def_ge4"]))

print("\nfw - plain gap   OLD: haiku {} / opus {}".format(
    summary["fw_minus_plain_gap"]["old_mean_def"]["haiku_fw_minus_plain"],
    summary["fw_minus_plain_gap"]["old_mean_def"]["opus_fw_minus_plain"]))
print("fw - plain gap   NEW: haiku {} / opus {}".format(
    summary["fw_minus_plain_gap"]["new_mean_def"]["haiku_fw_minus_plain"],
    summary["fw_minus_plain_gap"]["new_mean_def"]["opus_fw_minus_plain"]))
print("\nnew def>=4 wrong fraction: {} (old rubric was 0.703)".format(
    summary["new_def_ge4_wrong_fraction"]))
print("verdicts:", len(verdicts))
