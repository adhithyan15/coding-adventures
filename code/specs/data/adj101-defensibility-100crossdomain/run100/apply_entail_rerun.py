#!/usr/bin/env python3
"""ADJ101 — apply the adversarial entailment verdicts and re-run the engine.

A dispositive slot the adversary ruled LEAP was NOT actually established by the scenario -> we null it
(treat as not-stated). The engine then returns INDETERMINATE structurally, naming the un-grounded slot.

Measures both directions vs the baseline 100-run:
  - FIX: do the over-reads (TAX-4, CON-5, ...) now abstain? does underdetermined-abstention rise?
  - COST: do any clean-determinate items now WRONGLY abstain (adversary too aggressive)?

Reads: items_100.json, extractions100.json, check_map.json, entail_verdicts.json.
Run: python3 apply_entail_rerun.py
"""
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "adj86-defensibility-benchmark"))
import provenance_engine as PE  # noqa: E402

items = json.load(open(os.path.join(HERE, "items_100.json")))["items"]
ex = {e["idx"]: json.loads(json.dumps(e)) for e in json.load(open(os.path.join(HERE, "extractions100.json")))}
cmap = {c["check_id"]: c for c in json.load(open(os.path.join(HERE, "check_map.json")))}
verdicts = {v["check_id"]: v for v in json.load(open(os.path.join(HERE, "entail_verdicts.json")))}

# LEAP slots, grouped by item idx
leap_by_idx = defaultdict(list)
n_leap = n_entailed = 0
for cid, v in verdicts.items():
    c = cmap[cid]
    if v["verdict"] == "LEAP":
        leap_by_idx[c["idx"]].append(c["slot"])
        n_leap += 1
    else:
        n_entailed += 1


def family(s):
    s = (s or "").upper()
    if s.startswith("UNVERIFIED"):
        return "UNVERIFIED-RULEBOOK"
    for f in ("INDETERMINATE", "CONFLICT", "DETERMINATE"):
        if s.startswith(f):
            return f
    return s


def run(idx, gated):
    e = ex[idx]
    ir = json.loads(json.dumps(e["input_ir"]))
    if gated:
        for slot in leap_by_idx.get(idx, []):
            ir.get("slots", {}).pop(slot, None)   # un-entailed dispositive fact -> treat as not-stated
    res = PE.adjudicate(ir, e["rulebook_ir"], items[idx]["scenario"], items[idx]["policy"], e.get("justifications", []))
    return family(res["verdict"])


rows = []
for k, it in enumerate(items):
    if k not in ex:
        continue
    base = run(k, False)
    gat = run(k, True)
    rows.append({"idx": k, "id": it["id"], "domain": it["domain"], "stratum": it["stratum"],
                 "gold": it["gold_verdict"], "base": base, "gated": gat,
                 "leap_slots": leap_by_idx.get(k, [])})


def cnt(p):
    return sum(1 for r in rows if p(r))


und = [r for r in rows if r["stratum"] == "underdetermined-baited"]
clean = [r for r in rows if r["stratum"] == "clean-determinate"]


def match(r, key):
    return r[key] == r["gold"]


summary = {
    "entailment_verdicts": {"ENTAILED": n_entailed, "LEAP": n_leap, "total": n_entailed + n_leap},
    "verdict_family_match": {"baseline": f"{cnt(lambda r: match(r,'base'))}/{len(rows)}",
                             "gated": f"{cnt(lambda r: match(r,'gated'))}/{len(rows)}"},
    "underdetermined -> INDETERMINATE": {
        "baseline": f"{sum(1 for r in und if r['base']=='INDETERMINATE')}/{len(und)}",
        "gated": f"{sum(1 for r in und if r['gated']=='INDETERMINATE')}/{len(und)}"},
    "clean-determinate -> DETERMINATE (watch for false abstentions)": {
        "baseline": f"{sum(1 for r in clean if r['base']=='DETERMINATE')}/{len(clean)}",
        "gated": f"{sum(1 for r in clean if r['gated']=='DETERMINATE')}/{len(clean)}"},
}

# items whose verdict CHANGED under the gate
changed = [r for r in rows if r["base"] != r["gated"]]
json.dump({"summary": summary, "changed": changed, "rows": rows},
          open(os.path.join(HERE, "entail_gated_results.json"), "w"), ensure_ascii=False, indent=1)

print("\n=== ADJ101 — adversarial entailment gate, applied ===\n")
print(json.dumps(summary, indent=1))
print(f"\n{len(changed)} items changed verdict under the gate:")
for r in changed:
    tag = ""
    if r["stratum"] == "underdetermined-baited" and r["base"] != "INDETERMINATE" and r["gated"] == "INDETERMINATE":
        tag = "  <- FIX (now abstains)"
    if r["stratum"] == "clean-determinate" and r["base"] == "DETERMINATE" and r["gated"] != "DETERMINATE":
        tag = "  <- COST (false abstention)"
    print(f"  {r['id']:10} {r['stratum']:22} {r['base']:14} -> {r['gated']:14} gold={r['gold']:13} leap={r['leap_slots']}{tag}")
