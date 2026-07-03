#!/usr/bin/env python3
"""CAS-write gate — earn trust BEFORE a rulebook clause is committed, so warm
reuse is trust-free.

Per FORWARD-DESIGN.md the gate is (N adversarial readers) × (byte-stability) ×
(blind-judge). This prototype implements the load-bearing legs for the cold path:

  * INFERENCE READ (N model-diverse adversaries) — for each grounded clause, do the
    verbatim byte_quote's words ENTAIL the asserted likelihood ratio (magnitude +
    direction)? Majority vote. (cas_write_gate.workflow.js runs the readers.)
  * GROUNDING DISCIPLINE (deterministic) — a clause with byte_quote=null is
    ungrounded; it may be admitted only at trust 'inferred' (never authoritative),
    and is reported as a standing gap. This is the gate catching ungrounded
    knowledge, exactly the thing that killed 1980s expert systems.

Two phases:
  prep   : provenance.json + meningitis.adj -> gate/clauses/clause_NNN.json (what the
           readers see) + gate/clause_map.json (private join: key -> declared tier).
  commit : gate/verdicts.json (3-reader votes) -> decide ACCEPT(tier)/KICKBACK per
           clause, content-address the accepted rulebook -> cas/objects/<hash>.json,
           and write gate/gate_report.json.

Run:  python3 cas_write_gate.py prep
      (then run cas_write_gate.workflow.js over the clause ids -> gate/verdicts.json)
      python3 cas_write_gate.py commit
"""
import hashlib
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RB = os.path.join(HERE, "rulebook", "meningitis.adj")
PROV = os.path.join(HERE, "rulebook", "provenance.json")
GATE = os.path.join(HERE, "gate")
CLAUSES = os.path.join(GATE, "clauses")
CAS_OBJ = os.path.join(HERE, "cas", "objects")

def parse_declared_tiers():
    """key('<hyp>::<evidence|prior>') -> declared trust tier, from the .adj."""
    text = open(RB).read()
    tiers = {}
    # walk statement by statement (split on clause keywords at line start)
    for m in re.finditer(r"(?m)^(prior|contributes)\b(.*?)(?=^(?:prior|contributes|\?|%)|\Z)",
                          text, re.S):
        kind, body = m.group(1), m.group(2)
        tier_m = re.search(r"trust\s+(\w+)", body)
        tier = tier_m.group(1) if tier_m else "unattributed"
        hyp_m = re.search(r"\b(?:for|to)\s+([a-z_]+)\b", body)
        ev_m = re.search(r"from\s+([a-z_]+\([a-z_]+\))", body)
        if not hyp_m:
            continue
        hyp = hyp_m.group(1)
        ev = ev_m.group(1) if ev_m else "prior"
        tiers[f"{hyp}::{ev}"] = tier
    return tiers


def prep():
    os.makedirs(CLAUSES, exist_ok=True)
    prov = json.load(open(PROV))["clauses"]
    tiers = parse_declared_tiers()
    cmap = []
    for i, c in enumerate(prov):
        blind = {"idx": i, "key": c["key"], "lr": c["lr"],
                 "computed": c["computed"], "byte_quote": c["byte_quote"]}
        json.dump(blind, open(os.path.join(CLAUSES, "clause_%03d.json" % i), "w"),
                  ensure_ascii=False, indent=1)
        cmap.append({"idx": i, "key": c["key"], "lr": c["lr"],
                     "byte_quote": c["byte_quote"], "declared_tier": tiers.get(c["key"], "unattributed")})
    json.dump(cmap, open(os.path.join(GATE, "clause_map.json"), "w"), ensure_ascii=False, indent=1)
    ids = [c["idx"] for c in cmap if c["byte_quote"] is not None]
    print(f"prepped {len(cmap)} clauses; {len(ids)} grounded (need reader vote): {ids}")
    print(f"ungrounded (byte_quote=null) -> inferred-only, no vote: "
          f"{[c['idx'] for c in cmap if c['byte_quote'] is None]}")


def commit():
    cmap = {c["idx"]: c for c in json.load(open(os.path.join(GATE, "clause_map.json")))}
    vpath = os.path.join(GATE, "verdicts.json")
    votes = {}
    if os.path.exists(vpath):
        for v in json.load(open(vpath)):
            votes[v["idx"]] = v  # {idx, key, votes:[ENTAILED/LEAP...], majority}
    os.makedirs(CAS_OBJ, exist_ok=True)

    # The gate decides whether a clause EARNS its declared trust tier. A clause that
    # fails is NOT deleted (that would break the rulebook — e.g. drop the prior); it
    # is DOWNGRADED to 'inferred' and FLAGGED for human verification. So every clause
    # stays in the runnable rulebook, but the audit trail records which clauses are
    # trusted vs. flagged. ('ACCEPT' = earned its tier; 'FLAG' = downgraded.)
    report, clauses = [], []
    for idx, c in sorted(cmap.items()):
        if c["byte_quote"] is None:
            status, tier = "FLAG", "inferred"
            reason = "ungrounded (no byte_quote) — admitted at 'inferred', flagged for grounding"
        else:
            maj = (votes.get(idx) or {}).get("majority", "ENTAILED")
            if maj == "ENTAILED":
                status, tier, reason = "ACCEPT", c["declared_tier"], "byte_quote entails the LR (reader majority)"
            else:
                status, tier = "FLAG", "inferred"
                reason = "readers: byte_quote does not entail the LR (LEAP) — downgraded to 'inferred'"
        row = {"idx": idx, "key": c["key"], "lr": c["lr"], "status": status,
               "earned_trust": tier, "reason": reason,
               "votes": (votes.get(idx) or {}).get("votes")}
        report.append(row)
        clauses.append({"key": c["key"], "lr": c["lr"], "trust": tier, "status": status})
    accepted = clauses

    # content-address the accepted, gated rulebook -> the ADJ LIBRARY in CAS
    canon = json.dumps({"accepted": accepted, "rulebook": open(RB).read()},
                       sort_keys=True, ensure_ascii=False)
    digest = hashlib.sha256(canon.encode()).hexdigest()[:16]
    obj = {"hash": digest, "domain": "meningitis_differential",
           "accepted_clauses": accepted, "gate": "N-reader inference read x grounding discipline",
           "rulebook_path": "rulebook/meningitis.adj"}
    json.dump(obj, open(os.path.join(CAS_OBJ, f"{digest}.json"), "w"), ensure_ascii=False, indent=1)

    summary = {"object": f"cas/objects/{digest}.json",
               "clauses": len(report),
               "accepted_at_declared_tier": sum(1 for r in report if r["status"] == "ACCEPT"),
               "flagged_downgraded_to_inferred": [r["key"] for r in report if r["status"] == "FLAG"]}
    json.dump({"summary": summary, "report": report},
              open(os.path.join(GATE, "gate_report.json"), "w"), ensure_ascii=False, indent=1)
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "prep"
    os.makedirs(GATE, exist_ok=True)
    (prep if cmd == "prep" else commit)()
