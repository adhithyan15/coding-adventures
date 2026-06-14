#!/usr/bin/env python3
"""ADJ101 100-run — engine over the 100 extractions + per-domain/per-stratum aggregate.

Reuses the ADJ86 provenance engine. The model did ONLY extraction; the engine owns the verdict and
returns INDETERMINATE structurally when a dispositive slot is missing.

Reads: items_100.json + extractions100.json. Run: python3 run_engine100.py
"""
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "adj86-defensibility-benchmark"))
import provenance_engine as PE  # noqa: E402

items = json.load(open(os.path.join(HERE, "items_100_vetted.json")))["items"]
ex = {e["idx"]: e for e in json.load(open(os.path.join(HERE, "extractions100.json")))}


def family(v):
    v = (v or "").upper()
    if v.startswith("UNVERIFIED"):
        return "UNVERIFIED-RULEBOOK"
    for f in ("INDETERMINATE", "CONFLICT", "DETERMINATE"):
        if v.startswith(f):
            return f
    return v


rows = []
for k, it in enumerate(items):
    e = ex.get(k)
    if not e:
        rows.append({"idx": k, "id": it["id"], "domain": it["domain"], "stratum": it["stratum"], "extracted": False})
        continue
    try:
        res = PE.adjudicate(e["input_ir"], e["rulebook_ir"], it["scenario"], it["policy"], e.get("justifications", []))
        fam = family(res["verdict"])
        rows.append({
            "idx": k, "id": it["id"], "domain": it["domain"], "stratum": it["stratum"], "extracted": True,
            "engine_family": fam, "gold_family": it["gold_verdict"], "verdict_match": fam == it["gold_verdict"],
            "byte_accounting_ok": res["byte_accounting_ok"], "hallucinated_rules": res["hallucinated_rules"],
            "assumptions": res["assumptions"], "missing_slots": res.get("missing_slots_that_block", []),
        })
    except Exception as exn:  # noqa: BLE001
        rows.append({"idx": k, "id": it["id"], "domain": it["domain"], "stratum": it["stratum"],
                     "extracted": True, "engine_error": str(exn)[:120]})

ok = [r for r in rows if r.get("extracted") and not r.get("engine_error")]
n = len(ok)


def frac(p, rs=ok):
    m = [r for r in rs if p(r)]
    return f"{len(m)}/{len(rs)}" if rs else "0/0"


und = [r for r in ok if r["stratum"] == "underdetermined-baited"]
det = [r for r in ok if r["stratum"] != "underdetermined-baited"]

agg = {
    "n_items": len(items), "extracted": f"{n}/{len(items)}",
    "engine_errors": sum(1 for r in rows if r.get("engine_error")),
    "byte_accounting_clean": frac(lambda r: r["byte_accounting_ok"]),
    "no_hallucinated_rules": frac(lambda r: not r["hallucinated_rules"]),
    "verdict_family_match_overall": frac(lambda r: r["verdict_match"]),
    "underdetermined -> INDETERMINATE (abstain, not fabricate)":
        frac(lambda r: r["engine_family"] == "INDETERMINATE", und),
    "determinate strata -> DETERMINATE": frac(lambda r: r["engine_family"] == "DETERMINATE", det),
}

# per-domain
per_domain = {}
by_dom = defaultdict(list)
for r in ok:
    by_dom[r["domain"]].append(r)
for d, rs in sorted(by_dom.items()):
    per_domain[d] = {
        "n": len(rs),
        "byte_clean": sum(1 for r in rs if r["byte_accounting_ok"]),
        "verdict_match": sum(1 for r in rs if r["verdict_match"]),
    }

out = {"aggregate": agg, "per_domain": per_domain, "rows": rows}
json.dump(out, open(os.path.join(HERE, "run100_results.json"), "w"), ensure_ascii=False, indent=1)

print("\n=== ADJ101 — 100-item framework run (rule-engine arm) ===\n")
print(json.dumps(agg, indent=1))
print("\nper-domain (byte_clean / verdict_match out of n):")
for d, s in per_domain.items():
    print(f"  {d:24} byte_clean {s['byte_clean']:>2}/{s['n']:<2}  verdict_match {s['verdict_match']:>2}/{s['n']}")
# surface mismatches for inspection
mism = [r for r in ok if not r["verdict_match"]]
if mism:
    print(f"\n{len(mism)} verdict mismatches (engine vs gold) — for inspection:")
    for r in mism[:20]:
        print(f"  {r['id']:10} {r['stratum']:22} engine={r['engine_family']:14} gold={r['gold_family']}"
              + (f"  missing={r['missing_slots']}" if r.get("missing_slots") else ""))
