#!/usr/bin/env python3
"""ADJ101 — build the adversarial-entailment check list.

For each item, every DISPOSITIVE slot (one referenced by some rule's `when` and present in the
input-IR) gets an independent adversarial entailment check: does the SCENARIO actually establish this
slot's value, or did the extractor over-read the bytes / assert an unstated assumption?

We check the slot regardless of its self-reported stated/inferred label — the whole point is to stop
trusting that label. Writes one blind file per check for the adversary workflow to read.

Run: python3 build_entailment_checks.py
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CHECKS = os.path.join(HERE, "checks")
os.makedirs(CHECKS, exist_ok=True)

items = json.load(open(os.path.join(HERE, "items_100.json")))["items"]
ex = {e["idx"]: e for e in json.load(open(os.path.join(HERE, "extractions100.json")))}

checks = []
cid = 0
for k, it in enumerate(items):
    e = ex.get(k)
    if not e:
        continue
    slots = e["input_ir"].get("slots", {})
    rule_slots = set()
    for r in e["rulebook_ir"].get("rules", []):
        rule_slots |= set((r.get("when") or {}).keys())
    for name in rule_slots & set(slots):           # dispositive AND present
        s = slots[name]
        if s.get("value") is None:
            continue
        blind = {
            "check_id": cid, "idx": k, "slot": name,
            "claimed_value": s.get("value"),
            "scenario": it["scenario"],
            "cited_span": s.get("span") or s.get("basis_span") or "",
        }
        with open(os.path.join(CHECKS, "check_%03d.json" % cid), "w") as f:
            json.dump(blind, f, ensure_ascii=False, indent=1)
        checks.append({"check_id": cid, "idx": k, "id": it["id"], "slot": name})
        cid += 1

json.dump(checks, open(os.path.join(HERE, "check_map.json"), "w"), indent=1)
print("dispositive-slot checks:", len(checks), "over", len({c['idx'] for c in checks}), "items")
print("indices:", json.dumps([c["check_id"] for c in checks]))
