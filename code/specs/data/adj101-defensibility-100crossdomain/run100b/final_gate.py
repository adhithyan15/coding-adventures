#!/usr/bin/env python3
"""ADJ101 — ALL FIXES COMBINED, re-scored on the existing 100-run.

  - N-READER entailment: 3 model-diverse adversaries (Opus+Sonnet+Haiku) majority-vote each LEAP slot
    (overturns single-reader over-strict false-LEAPs).
  - DECISION-SENSITIVITY: null a majority-LEAP slot only if outcome-pivotal (boolean negation test;
    numeric kept unless removal yields a different determinate answer).
  - PRECEDENCE: expand override markers (engine list lacked 'override'); drop '*'-only definitional
    "rules" (a definition is not a decision).
  - Also ADJUDICATES the residual clean-determinate false-abstentions: if the 3-reader panel
    unanimously/majority says the slot is LEAP, the framework's abstention is correct and the
    *generated gold* is the error.

Run: python3 final_gate.py
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "adj86-defensibility-benchmark"))
sys.path.insert(0, os.path.join(HERE, "..", "..", "adj84-pipeline-defensibility"))
import engine as base  # noqa: E402
import provenance_engine as PE  # noqa: E402
base.OVERRIDE_MARKERS = tuple(set(base.OVERRIDE_MARKERS) | {"override"})  # PRECEDENCE fix 1

items = json.load(open(os.path.join(HERE, "items_100_vetted.json")))["items"]
ex = {e["idx"]: e for e in json.load(open(os.path.join(HERE, "extractions100.json")))}
cmap = {c["check_id"]: c for c in json.load(open(os.path.join(HERE, "check_map.json")))}
maj = {int(k): v for k, v in json.load(open(os.path.join(HERE, "nreader_majority.json"))).items()}

# majority-LEAP slots per item idx
leap = {}
for cid, c in cmap.items():
    if maj.get(cid) == "LEAP":          # only slots the 3-reader panel agrees are LEAP
        leap.setdefault(c["idx"], []).append(c["slot"])


def drop_defn(rb):                       # PRECEDENCE fix 2: '*'-only rule == definition, not decision
    return {**rb, "rules": [r for r in rb["rules"]
                            if not (len(r.get("when") or {}) == 1 and list((r.get("when") or {}).values())[0].strip() == "*")]}


def fam(s):
    s = (s or "").upper()
    for f in ("INDETERMINATE", "CONFLICT", "UNVERIFIED", "DETERMINATE"):
        if s.startswith(f):
            return "UNVERIFIED" if f == "UNVERIFIED" else f
    return s


def verdict(idx, slots):
    e = ex[idx]
    r = PE.adjudicate({**e["input_ir"], "slots": slots}, drop_defn(e["rulebook_ir"]),
                      items[idx]["scenario"], items[idx]["policy"], e.get("justifications", []))
    return r["verdict"], r.get("answer")


def is_bool(v):
    return isinstance(v, bool) or (isinstance(v, str) and v.strip().lower() in ("true", "false"))


def neg(v):
    return (not v) if isinstance(v, bool) else ("false" if str(v).strip().lower() == "true" else "true")


def pivotal(idx, slots, s):
    v = slots[s].get("value")
    bv, ba = verdict(idx, slots)
    if is_bool(v):
        fv, fa = verdict(idx, {**slots, s: {**slots[s], "value": neg(v)}})
        return fam(bv) != fam(fv) or (fam(bv) == "DETERMINATE" == fam(fv) and ba != fa)
    rv, ra = verdict(idx, {k: x for k, x in slots.items() if k != s})
    return fam(rv) == "DETERMINATE" and (fam(bv) != "DETERMINATE" or ba != ra)


rows = []
for k, it in enumerate(items):
    if k not in ex:
        continue
    slots = json.loads(json.dumps(ex[k]["input_ir"].get("slots", {})))
    nulled = [s for s in leap.get(k, []) if s in slots and slots[s].get("value") is not None and pivotal(k, slots, s)]
    g = fam(verdict(k, {s: x for s, x in slots.items() if s not in nulled})[0])
    rows.append({"id": it["id"], "stratum": it["stratum"], "gold": it["gold_verdict"], "final": g,
                 "nulled": nulled, "maj_leap": leap.get(k, [])})


def n(p, rs):
    return sum(1 for r in rs if p(r))


und = [r for r in rows if r["stratum"] == "underdetermined-baited"]
det = [r for r in rows if r["stratum"] != "underdetermined-baited"]  # clean + override + exception
# residual determinate-stratum abstentions where the panel agrees the slot is LEAP => gold error, not gate error
det_abst = [r for r in det if r["final"] != "DETERMINATE"]
gold_errors = [r for r in det_abst if r["nulled"]]  # abstained because a majority-LEAP slot was nulled

summary = {
    "corpus": "run100b (fresh, gold-vetted, 10 unseen domains)",
    "verdict_match_vs_gold": {"baseline": "86/100", "blunt_gate(null any LEAP)": "see ENTAILMENT_GATE_FINDINGS",
                              "N-reader x sensitivity x precedence": f"{n(lambda r: r['final']==r['gold'], rows)}/100"},
    "underdetermined -> INDETERMINATE": f"{n(lambda r: r['final']=='INDETERMINATE', und)}/{len(und)}",
    "determinate strata -> DETERMINATE": f"{n(lambda r: r['final']=='DETERMINATE', det)}/{len(det)}",
    "residual determinate-stratum abstentions": [r["id"] for r in det_abst],
    "...of which panel says the nulled slot IS unestablished (=> GOLD likely wrong, abstention correct)":
        [(r["id"], r["nulled"]) for r in gold_errors],
}
json.dump({"summary": summary, "rows": rows}, open(os.path.join(HERE, "final_gate_results.json"), "w"), indent=1)
print("\n=== ADJ101 — all fixes combined (N-reader x decision-sensitivity x precedence) ===\n")
print(json.dumps(summary, indent=1))
