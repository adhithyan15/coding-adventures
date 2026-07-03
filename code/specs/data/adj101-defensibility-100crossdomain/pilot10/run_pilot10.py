#!/usr/bin/env python3
"""ADJ101 10-item pilot — execute each emitted program through the provenance executor + aggregate.

Reads:
  items_compute10.json     (the corpus: quantity_spans + gold held aside)
  emissions10.json         (the translator workflow output: per-item {facts, discarded, program})
Writes:
  pilot10_results.json     (per-item audit + aggregate)
Run:  python3 run_pilot10.py
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import provenance_program as P  # noqa: E402

items = {it["id"]: it for it in json.load(open(os.path.join(HERE, "items_compute10.json")))["items"]}
emissions = {e["id"]: e for e in json.load(open(os.path.join(HERE, "emissions10.json")))}

rows = []
for iid, it in items.items():
    em = emissions.get(iid)
    if not em or em.get("_error"):
        rows.append({"id": iid, "emitted": False})
        continue
    out = P.adjudicate_program(it["quantity_spans"], em, gold=it["gold_answer"], tolerance=it.get("tolerance", 0))
    rows.append({
        "id": iid, "domain": it["domain"], "emitted": True,
        "exec_ok": out["exec_ok"], "result": out["result"], "gold": it["gold_answer"],
        "correct": out["correct"],                 # informational
        "auditable": out["auditable"],
        "fabrications": out["fabrications"], "unfaithful_facts": out["unfaithful_facts"],
        "missing_coverage": out["missing_coverage"], "magic_numbers": out["magic_numbers"],
        "error_locus": out["error_locus"], "stderr": out["stderr"],
        "n_facts": len(em.get("facts", {})), "n_discarded": len(em.get("discarded", [])),
    })


def frac(p):
    n = sum(1 for r in rows if r.get("emitted"))
    return f"{sum(1 for r in rows if p(r))}/{n}" if n else "0/0"


agg = {
    "n_items": len(items),
    "programs_emitted": frac(lambda r: r.get("emitted")),
    "programs_executed": frac(lambda r: r.get("exec_ok")),
    "auditable": frac(lambda r: r.get("auditable")),
    "provenance_clean_breakdown": {
        "no_fabrications": frac(lambda r: r.get("emitted") and not r.get("fabrications")),
        "no_magic_numbers": frac(lambda r: r.get("emitted") and not r.get("magic_numbers")),
        "coverage_complete": frac(lambda r: r.get("emitted") and not r.get("missing_coverage")),
    },
    "correct_informational": frac(lambda r: r.get("correct") is True),
    # the rescored-paradigm metric: of the items that did NOT cleanly succeed, how many are localized
    # (the audit names where to look: an unfaithful fact, a fabrication, or an exec error)?
    "not_clean": frac(lambda r: r.get("emitted") and not (r.get("correct") and r.get("auditable"))),
    "not_clean_AND_localized": frac(lambda r: r.get("emitted") and not (r.get("correct") and r.get("auditable")) and (
        r.get("unfaithful_facts") or r.get("fabrications") or r.get("stderr"))),
}

json.dump({"aggregate": agg, "rows": rows}, open(os.path.join(HERE, "pilot10_results.json"), "w"),
          ensure_ascii=False, indent=1)

print("\n=== ADJ101 10-item program-emission pilot ===\n")
hdr = "{:<7} {:<14} {:>5} {:>10} {:>10} {:>6} {:>6}".format("id", "domain", "exec", "result", "gold", "audit", "corr")
print(hdr); print("-" * len(hdr))
for r in rows:
    if not r.get("emitted"):
        print("{:<7} {:<14} NOT EMITTED".format(r["id"], "")); continue
    res = r["result"]
    res_s = str(round(res, 3)) if isinstance(res, (int, float)) else str(res)[:10]
    print("{:<7} {:<14} {:>5} {:>10} {:>10} {:>6} {:>6}".format(
        r["id"], r["domain"][:14], "ok" if r["exec_ok"] else "FAIL",
        res_s, str(r["gold"]), "Y" if r["auditable"] else "n", "Y" if r["correct"] else "n"))
    flags = []
    if r["fabrications"]: flags.append(f"fabricated={r['fabrications']}")
    if r["unfaithful_facts"]: flags.append(f"unfaithful={r['unfaithful_facts']}")
    if r["missing_coverage"]: flags.append(f"dropped={r['missing_coverage']}")
    if r["magic_numbers"]: flags.append(f"magic={r['magic_numbers']}")
    if r["stderr"]: flags.append(f"exec_err={r['stderr'][:60]}")
    if flags: print("        -> " + "; ".join(flags))

print("\naggregate:", json.dumps(agg, indent=1))
