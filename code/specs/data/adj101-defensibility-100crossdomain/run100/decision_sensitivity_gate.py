#!/usr/bin/env python3
"""ADJ101 — entailment x DECISION-SENSITIVITY gate (the fix for the blunt-gate over-abstention).

The blunt gate (null any LEAP dispositive slot) over-abstained: 88->74, because it nulls LEAP slots
whose imprecision doesn't change the verdict (MED-2: 'not in last 12h' is LEAP re exact value, but the
rule needs >=4h which any >=12 satisfies). The fix (ADJ65 decision-sensitivity): a LEAP slot triggers
abstention ONLY IF it is OUTCOME-PIVOTAL.

Pivotality test per LEAP slot s with model value v:
  - boolean v: compare engine verdict at s=v vs s=not(v). Pivotal iff the verdict family differs OR
    (both DETERMINATE and the answers differ). [catches the boolean fabrications TAX-4, TAX-6]
  - numeric/string v: compare verdict with s present vs s removed. Pivotal ONLY IF removal yields a
    DIFFERENT DETERMINATE answer (not merely INDETERMINATE). [keeps MED-2/IMM-2/CON-2: removing the
    over-precise value goes to INDETERMINATE, which is not 'a different determinate answer' -> keep]

Only pivotal LEAP slots are nulled; the engine then abstains naming them. Non-pivotal LEAPs are kept.
Reuses the 100-run extractions + the adversarial entailment verdicts (no new model calls).
Run: python3 decision_sensitivity_gate.py
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "adj86-defensibility-benchmark"))
import provenance_engine as PE  # noqa: E402

items = json.load(open(os.path.join(HERE, "items_100.json")))["items"]
ex = {e["idx"]: e for e in json.load(open(os.path.join(HERE, "extractions100.json")))}
cmap = {c["check_id"]: c for c in json.load(open(os.path.join(HERE, "check_map.json")))}
verds = {v["check_id"]: v for v in json.load(open(os.path.join(HERE, "entail_verdicts.json")))}

# LEAP slots per item
leap = {}
for cid, v in verds.items():
    if v["verdict"] == "LEAP":
        c = cmap[cid]
        leap.setdefault(c["idx"], []).append(c["slot"])


def fam(s):
    s = (s or "").upper()
    if s.startswith("UNVERIFIED"):
        return "UNVERIFIED"
    for f in ("INDETERMINATE", "CONFLICT", "DETERMINATE"):
        if s.startswith(f):
            return f
    return s


def verdict(idx, slots):
    e = ex[idx]
    ir = {**e["input_ir"], "slots": slots}
    r = PE.adjudicate(ir, e["rulebook_ir"], items[idx]["scenario"], items[idx]["policy"], e.get("justifications", []))
    return r["verdict"], r.get("answer")


def is_bool(v):
    return isinstance(v, bool) or (isinstance(v, str) and v.strip().lower() in ("true", "false"))


def neg(v):
    if isinstance(v, bool):
        return not v
    return "false" if str(v).strip().lower() == "true" else "true"


def pivotal(idx, slots, s):
    v = slots[s].get("value")
    base_v, base_a = verdict(idx, slots)
    if is_bool(v):
        flipped = {**slots, s: {**slots[s], "value": neg(v)}}
        f_v, f_a = verdict(idx, flipped)
        return fam(base_v) != fam(f_v) or (fam(base_v) == "DETERMINATE" == fam(f_v) and base_a != f_a)
    # numeric / string: pivotal only if REMOVING it yields a DIFFERENT DETERMINATE answer
    removed = {k: x for k, x in slots.items() if k != s}
    r_v, r_a = verdict(idx, removed)
    return fam(r_v) == "DETERMINATE" and (fam(base_v) != "DETERMINATE" or base_a != r_a)


rows = []
for k, it in enumerate(items):
    if k not in ex:
        continue
    slots = json.loads(json.dumps(ex[k]["input_ir"].get("slots", {})))
    base = fam(verdict(k, slots)[0])
    nulled = []
    for s in leap.get(k, []):
        if s in slots and slots[s].get("value") is not None and pivotal(k, slots, s):
            nulled.append(s)
    gated_slots = {s: x for s, x in slots.items() if s not in nulled}
    gated = fam(verdict(k, gated_slots)[0])
    rows.append({"id": it["id"], "domain": it["domain"], "stratum": it["stratum"], "gold": it["gold_verdict"],
                 "base": base, "gated": gated, "leap": leap.get(k, []), "nulled_pivotal": nulled})


def c(p, rs):
    return sum(1 for r in rs if p(r))


und = [r for r in rows if r["stratum"] == "underdetermined-baited"]
clean = [r for r in rows if r["stratum"] == "clean-determinate"]
summary = {
    "verdict_match": {
        "baseline_88_blunt_74": "(reference)",
        "sensitivity_gated": f"{c(lambda r: r['gated']==r['gold'], rows)}/{len(rows)}"},
    "underdetermined -> INDETERMINATE": {
        "baseline": f"{c(lambda r: r['base']=='INDETERMINATE', und)}/{len(und)}",
        "gated": f"{c(lambda r: r['gated']=='INDETERMINATE', und)}/{len(und)}"},
    "clean-determinate -> DETERMINATE": {
        "baseline": f"{c(lambda r: r['base']=='DETERMINATE', clean)}/{len(clean)}",
        "gated": f"{c(lambda r: r['gated']=='DETERMINATE', clean)}/{len(clean)}"},
}
json.dump({"summary": summary, "rows": rows}, open(os.path.join(HERE, "sensitivity_gated_results.json"), "w"), indent=1)

print("\n=== ADJ101 — entailment x decision-sensitivity gate ===\n")
print(json.dumps(summary, indent=1))
fixes = [r for r in rows if r["stratum"] == "underdetermined-baited" and r["base"] != "INDETERMINATE" and r["gated"] == "INDETERMINATE"]
kept = [r for r in rows if r["stratum"] == "clean-determinate" and r["leap"] and r["gated"] == "DETERMINATE"]
print(f"\nFIXED over-reads -> abstain ({len(fixes)}):", [r["id"] for r in fixes])
print(f"clean items with LEAP slots kept determinate (no false abstention):", [r['id'] for r in kept][:15])
bad = [r for r in rows if r["stratum"] == "clean-determinate" and r["base"] == "DETERMINATE" and r["gated"] != "DETERMINATE"]
print(f"residual false abstentions on clean-determinate ({len(bad)}):", [(r['id'], r['nulled_pivotal']) for r in bad])
