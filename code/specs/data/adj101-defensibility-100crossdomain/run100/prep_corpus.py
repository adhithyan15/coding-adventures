#!/usr/bin/env python3
"""ADJ101 100-run — flatten + validate the generated corpus into items_100.json.

Reads gen_corpus_raw.json (the gen_corpus.workflow.js output: per-domain {items:[...]}), validates
structural gold (stratum <-> verdict family), dedups ids, and writes items_100.json + a report.
Run: python3 prep_corpus.py
"""
import json
import os
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
raw = json.load(open(os.path.join(HERE, "gen_corpus_raw.json")))

STRATA = {"clean-determinate", "underdetermined-baited", "override-precedence", "exception-encoding"}
items, issues = [], []
seen = set()
for dom in raw:
    if dom.get("_error"):
        issues.append(f"domain {dom['domain']}: generation error")
        continue
    d = dom["domain"]
    for j, it in enumerate(dom.get("items", [])):
        iid = it.get("id") or f"{d[:3].upper()}-{j+1}"
        if iid in seen:
            iid = f"{d[:3].upper()}-{j+1}-{len(seen)}"
        seen.add(iid)
        st = it.get("stratum")
        gv = it.get("gold_verdict")
        # structural-gold invariant: underdetermined <-> INDETERMINATE; others -> DETERMINATE
        if st not in STRATA:
            issues.append(f"{iid}: bad stratum {st!r}")
        if st == "underdetermined-baited" and gv != "INDETERMINATE":
            issues.append(f"{iid}: underdetermined but gold {gv}")
        if st and st != "underdetermined-baited" and gv != "DETERMINATE":
            issues.append(f"{iid}: {st} but gold {gv}")
        for k in ("policy", "scenario", "question", "gold_answer_substring"):
            if not it.get(k):
                issues.append(f"{iid}: missing {k}")
        items.append({"id": iid, "domain": d, "stratum": st, "policy": it.get("policy", ""),
                      "scenario": it.get("scenario", ""), "question": it.get("question", ""),
                      "gold_verdict": gv, "gold_answer_substring": it.get("gold_answer_substring", "")})

report = {
    "n_items": len(items),
    "domains": dict(Counter(i["domain"] for i in items)),
    "strata": dict(Counter(i["stratum"] for i in items)),
    "gold_verdict": dict(Counter(i["gold_verdict"] for i in items)),
    "n_issues": len(issues), "issues": issues[:40],
}
json.dump({"items": items}, open(os.path.join(HERE, "items_100.json"), "w"), ensure_ascii=False, indent=1)
print(json.dumps(report, indent=1))
print(f"\nwrote items_100.json ({len(items)} items); {len(issues)} validation issues")
