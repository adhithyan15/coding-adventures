#!/usr/bin/env python3
"""E2 — aggregate the localize panel with bootstrap CIs, joined to the private
cell map, and fold in the fix/propagate panel.

Metrics (pre-registered, see 00-preregistration.md §4):
  localize_rate    = (hit + 0.5*partial) / n_wrong, per arm. Primary, RQ1.
  auditor_fooled   = fraction the auditor declared sound / affirmed a false premise.
  framework - plain delta with a 10k-resample bootstrap 95% CI over ITEMS (paired).

The bootstrap resamples ITEMS (not cells), preserving the within-item fw/plain
pairing — the matched design the pre-registration commits to.

Run: python3 aggregate.py
"""
import json
import os
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
cells = {c["idx"]: c for c in json.load(open(os.path.join(HERE, "items_e2.json")))}
results = json.load(open(os.path.join(HERE, "localize_results.json")))

SCORE = {"hit": 1.0, "partial": 0.5, "miss": 0.0, "n/a": None}


def arm_scale(cid):
    c = cells[cid]
    return c["arm"], c["scale"], c["item_id"], c["category"]


# join: per (scale) collect paired item -> {framework: score, plain: score, fooled}
rows = []
for r in results:
    cid = r["cid"]
    arm, scale, iid, cat = arm_scale(cid)
    loc = r["score"]["localization"]
    rows.append({"cid": cid, "arm": arm, "scale": scale, "item_id": iid, "category": cat,
                 "localization": loc, "loc_score": SCORE.get(loc),
                 "fooled": bool(r["score"].get("auditor_fooled"))})


def summarize(subset):
    """subset: list of rows. Returns per-arm localize_rate + fooled_rate + paired item deltas."""
    by_arm = defaultdict(list)
    for r in subset:
        if r["loc_score"] is not None:        # drop n/a (solution actually correct)
            by_arm[r["arm"]].append(r)
    out = {}
    for arm in ("framework", "plain"):
        rs = by_arm[arm]
        n = len(rs)
        out[arm] = {
            "n_wrong": n,
            "localize_rate": round(sum(x["loc_score"] for x in rs) / n, 4) if n else None,
            "hit": sum(1 for x in rs if x["localization"] == "hit"),
            "partial": sum(1 for x in rs if x["localization"] == "partial"),
            "miss": sum(1 for x in rs if x["localization"] == "miss"),
            "auditor_fooled": sum(1 for x in rs if x["fooled"]),
            "auditor_fooled_rate": round(sum(1 for x in rs if x["fooled"]) / n, 4) if n else None,
        }
    return out, by_arm


def paired_items(by_arm):
    """item_id -> (fw_score, plain_score) where both arms scored (not n/a)."""
    fw = {x["item_id"]: x["loc_score"] for x in by_arm["framework"]}
    pl = {x["item_id"]: x["loc_score"] for x in by_arm["plain"]}
    return {i: (fw[i], pl[i]) for i in fw.keys() & pl.keys()}


def bootstrap_delta(pairs, B=10000):
    """Paired bootstrap over items of mean(fw - plain). Deterministic LCG (no RNG import
    needed; reproducible)."""
    items = list(pairs.values())
    n = len(items)
    if n == 0:
        return None
    deltas = [fw - pl for fw, pl in items]
    point = sum(deltas) / n
    seed = 2654435761
    samples = []
    for _ in range(B):
        s = 0.0
        for _ in range(n):
            seed = (1103515245 * seed + 12345) & 0x7FFFFFFF
            s += deltas[seed % n]
        samples.append(s / n)
    samples.sort()
    lo = samples[int(0.025 * B)]
    hi = samples[int(0.975 * B)]
    return {"point_delta_fw_minus_plain": round(point, 4),
            "ci95": [round(lo, 4), round(hi, 4)], "n_items": n,
            "excludes_zero": bool(lo > 0 or hi < 0)}


report = {}
for label, sub in (("primary_haiku", [r for r in rows if r["scale"] == "haiku"]),
                   ("robustness_opus", [r for r in rows if r["scale"] == "opus"]),
                   ("pooled", rows)):
    summ, by_arm = summarize(sub)
    pairs = paired_items(by_arm)
    report[label] = {"per_arm": summ, "paired_localize_delta": bootstrap_delta(pairs)}

# fold in the fix/propagate panel if present
fp = os.path.join(HERE, "fix_propagate.json")
if os.path.exists(fp):
    report["fix_propagate"] = json.load(open(fp))

# leak-check echo
lc = os.path.join(HERE, "leak_check.json")
if os.path.exists(lc):
    report["leak_check"] = json.load(open(lc))

json.dump({"report": report, "rows": rows}, open(os.path.join(HERE, "aggregate.json"), "w"), indent=1)

# console headline
print("\n=== E2 correctability — localize panel ===\n")
for label in ("primary_haiku", "robustness_opus", "pooled"):
    r = report[label]
    fw, pl = r["per_arm"]["framework"], r["per_arm"]["plain"]
    d = r["paired_localize_delta"]
    print(f"[{label}]  framework localize={fw['localize_rate']} (h{fw['hit']}/p{fw['partial']}/m{fw['miss']}, "
          f"fooled {fw['auditor_fooled']}/{fw['n_wrong']})  |  "
          f"plain localize={pl['localize_rate']} (h{pl['hit']}/p{pl['partial']}/m{pl['miss']}, "
          f"fooled {pl['auditor_fooled']}/{pl['n_wrong']})")
    if d:
        print(f"            paired delta (fw-plain) = {d['point_delta_fw_minus_plain']} "
              f"CI95 {d['ci95']} (n={d['n_items']} items, excludes 0: {d['excludes_zero']})\n")
