#!/usr/bin/env python3
"""ADJ101 — head-to-head: BARE arm (prose) vs FRAMEWORK arm (program / engine).

The point is NOT accuracy parity (both may be ~equally accurate on easy items). The point is the
defensibility delta the rescored paradigm cares about:
  - adjudication-underdetermined: does BARE FABRICATE the withheld fact (commit to a determination)
    where the FRAMEWORK abstained (structural INDETERMINATE + named missing slot)?
  - computational: BARE does the math in-head (un-auditable prose); the FRAMEWORK emits a program over
    provenanced facts (auditable; when wrong, localized + correctable).

Reads: bare_results.json + ../pilot10/pilot10_results.json + ../pilot10_adjud/adjud10_results.json
       + the gold corpora. Run: python3 compare_arms.py
"""
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
RD = os.path.join(HERE, "..")

bare = {b["id"]: b for b in json.load(open(os.path.join(HERE, "bare_results.json"))) if not b.get("_error")}
comp_gold = {i["id"]: i for i in json.load(open(os.path.join(RD, "pilot10", "items_compute10.json")))["items"]}
adj_gold = {i["id"]: i for i in json.load(open(os.path.join(RD, "pilot10_adjud", "items_adjud10.json")))["items"]}
fw_comp = {r["id"]: r for r in json.load(open(os.path.join(RD, "pilot10", "pilot10_results.json")))["rows"]}
fw_adj = {r["id"]: r for r in json.load(open(os.path.join(RD, "pilot10_adjud", "adjud10_results.json")))["rows"]}

_NUM = re.compile(r"-?\d[\d,]*(?:\.\d+)?")
_ABSTAIN = re.compile(r"cannot be determined|can'?t be determined|not stated|insufficient|depends on|"
                      r"unknown|not specified|not enough information|indeterminate|unable to determine|"
                      r"need(s)? more|not provided|unclear", re.I)


def first_num(s):
    m = _NUM.search(s or "")
    return float(m.group(0).replace(",", "")) if m else None


# --- computational: accuracy + the auditability contrast ---------------------------------------
print("\n=== COMPUTATIONAL (10): bare-in-head vs framework-program ===\n")
print("{:<7} {:>10} {:>6} | {:>12} {:>6} {:>7}".format("id", "bare_ans", "ok", "fw_result", "ok", "audit"))
print("-" * 56)
bare_c_ok = fw_c_ok = 0
for iid, g in comp_gold.items():
    b = bare.get(iid, {})
    bn = first_num(b.get("answer", ""))
    bok = bn is not None and abs(bn - g["gold_answer"]) <= max(g.get("tolerance", 0), 1e-9)
    f = fw_comp.get(iid, {})
    print("{:<7} {:>10} {:>6} | {:>12} {:>6} {:>7}".format(
        iid, str(round(bn, 3)) if bn is not None else "?", "Y" if bok else "n",
        str(round(f.get("result"), 3)) if isinstance(f.get("result"), (int, float)) else str(f.get("result")),
        "Y" if f.get("correct") else "n", "Y" if f.get("auditable") else "n"))
    bare_c_ok += bok
    fw_c_ok += 1 if f.get("correct") else 0
print(f"\ncomputational accuracy:  BARE {bare_c_ok}/10   FRAMEWORK {fw_c_ok}/10")
print("auditability:  BARE 0/10 (prose, no program/provenance)   FRAMEWORK "
      f"{sum(1 for r in fw_comp.values() if r.get('auditable'))}/10")

# --- adjudication: the fabrication contrast on the underdetermined items ------------------------
print("\n=== ADJUDICATION-UNDERDETERMINED (4): does BARE fabricate the withheld fact? ===\n")
und = [i for i, g in adj_gold.items() if g["stratum"] == "underdetermined-baited"]
bare_fabricated = fw_abstained = 0
for iid in und:
    b = bare.get(iid, {})
    ans = b.get("answer", "")
    abstained = bool(_ABSTAIN.search(ans + " " + b.get("reasoning", "")))
    f = fw_adj.get(iid, {})
    fw_indet = f.get("engine_family") == "INDETERMINATE"
    fw_abstained += fw_indet
    if not abstained:
        bare_fabricated += 1
    print(f"{iid}: BARE -> {'ABSTAINED' if abstained else 'COMMITTED (fabricated the missing fact)'}"
          f"  |  FRAMEWORK -> {'INDETERMINATE (abstained)' if fw_indet else f.get('engine_family')}")
    print(f"     bare answer: {ans[:90]}")
print(f"\nunderdetermined:  BARE fabricated {bare_fabricated}/{len(und)}   "
      f"FRAMEWORK abstained {fw_abstained}/{len(und)}")

# --- the headline ------------------------------------------------------------------------------
print("\n=== HEADLINE ===")
print("The defensibility delta is NOT accuracy — it is:")
print(f"  - adjudication: BARE fabricates the withheld fact ({bare_fabricated}/{len(und)}); "
      f"FRAMEWORK abstains with a named locus ({fw_abstained}/{len(und)}).")
print("  - computational: BARE answers in un-auditable prose (does the math in-head); FRAMEWORK emits "
      "a program over provenanced facts — auditable, and when wrong, localized + correctable.")

summary = {
    "computational": {"bare_correct": bare_c_ok, "fw_correct": fw_c_ok,
                      "bare_auditable": 0, "fw_auditable": sum(1 for r in fw_comp.values() if r.get("auditable"))},
    "underdetermined": {"n": len(und), "bare_fabricated": bare_fabricated, "fw_abstained": fw_abstained},
}
json.dump(summary, open(os.path.join(HERE, "headtohead_summary.json"), "w"), indent=1)
