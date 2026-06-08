#!/usr/bin/env python3
"""ADJ86 — score the byte-cited justification check: byte-anchor + ENTAILED/LEAP discrimination.

For each inferred fact: (deterministic) the cited basis_span must be VERBATIM in the scenario
(byte-anchor); then the adversarial verdict ENTAILED (grounded by the bytes' meaning) vs LEAP
(needs world knowledge). Only LEAP (or a non-verbatim/absent basis) is a genuine ASSUMPTION;
ENTAILED inferred facts are grounded and must NOT be flagged. This is the fix for the
over-flagging the v2 judge caught (e.g. "four months" -> "<1 year").

Usage: python justify_eval.py <justify-results.json>
"""
from __future__ import annotations

import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FACTS = {(f["id"], f["slot"]): f for f in json.load(open(os.path.join(HERE, "facts.json")))}


def norm(s):
    return re.sub(r"\s+", " ", (s or "")).strip().lower()


def main():
    res = json.loads(open(sys.argv[1]).read())
    res = res.get("result", res)
    js = res["justifications"] if "justifications" in res else res

    print(f"  {'item':24} {'inferred slot':34} {'basis (verbatim?)':20} {'verdict':9} grounded?")
    print("-" * 104)
    entailed = leap = anchor_fail = 0
    out = []
    for j in js:
        f = FACTS[(j["id"], j["slot"])]
        scen = norm(f["scenario"])
        basis = j.get("basis_span")
        verbatim = bool(basis) and norm(basis) in scen
        if basis and not verbatim:
            anchor_fail += 1
        # genuine assumption iff LEAP, OR basis not verbatim/absent
        grounded = (j["verdict"] == "ENTAILED") and verbatim
        if grounded:
            entailed += 1
        else:
            leap += 1
        vb = "yes" if verbatim else ("NOT-IN-TEXT" if basis else "(none)")
        print(f"  {j['id'][:24]:24} {j['slot'][:34]:34} {vb:20} {j['verdict']:9} {'GROUNDED' if grounded else 'ASSUMPTION'}")
        out.append({**j, "basis_verbatim": verbatim, "grounded": grounded})

    n = len(js)
    print("-" * 104)
    print(f"  {n} inferred facts: GROUNDED (ENTAILED + verbatim basis) = {entailed}  |  ASSUMPTION (LEAP / no basis) = {leap}")
    print(f"  byte-anchor failures (cited basis not verbatim in scenario) = {anchor_fail}")
    print("\n  Key checks (the v2 over-flags vs the genuine leaps):")
    for (iid, slot) in [("LAW4-capgains-shortterm", "holding_period_less_than_one_year"),
                        ("MED1-specialist", "provider_is_specialist"),
                        ("MED5-er-copay", "copay_currency"), ("LAW6-books-duty", "currency")]:
        v = next((x for x in out if x["id"] == iid and x["slot"] == slot), None)
        if v:
            print(f"    {iid:24} {slot:34} -> {v['verdict']:9} ({'GROUNDED — no longer flagged' if v['grounded'] else 'ASSUMPTION — correctly flagged'})")
    json.dump(out, open(os.path.join(HERE, "justify_eval_results.json"), "w"), indent=2)


if __name__ == "__main__":
    main()
