#!/usr/bin/env python3
"""ADJ101 adjudication pilot — run each extracted IR through the deterministic engine + aggregate.

Reuses the ADJ86 provenance-complete engine (rulebook-span verification + assumption discipline),
which wraps the ADJ84 deterministic engine. The model did ONLY extraction; the ENGINE owns the
verdict and returns INDETERMINATE structurally when a dispositive slot is missing (abstain, not
fabricate). Scored on auditability/localizability; verdict-match is informational.

Reads: items_adjud10.json (gold) + emissions_adjud10.json (workflow output).
Run:   python3 run_adjud10.py
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "adj86-defensibility-benchmark"))
import provenance_engine as PE  # noqa: E402

items = {it["id"]: it for it in json.load(open(os.path.join(HERE, "items_adjud10.json")))["items"]}
emissions = {e["id"]: e for e in json.load(open(os.path.join(HERE, "emissions_adjud10.json")))}


def family(verdict: str) -> str:
    v = (verdict or "").upper()
    if v.startswith("UNVERIFIED"):
        return "UNVERIFIED-RULEBOOK"
    for fam in ("INDETERMINATE", "CONFLICT", "DETERMINATE"):
        if v.startswith(fam):
            return fam
    return v


rows = []
for iid, it in items.items():
    em = emissions.get(iid)
    if not em or em.get("_error"):
        rows.append({"id": iid, "extracted": False})
        continue
    try:
        res = PE.adjudicate(em["input_ir"], em["rulebook_ir"], it["scenario"], it["policy"],
                            em.get("justifications", []))
    except Exception as e:  # noqa: BLE001
        rows.append({"id": iid, "extracted": True, "engine_error": f"{type(e).__name__}: {e}"})
        continue
    eng_fam = family(res["verdict"])
    gold_fam = it["gold_verdict"]
    verdict_match = eng_fam == gold_fam
    # for DETERMINATE, also check the answer carries the gold substring (informational)
    ans = str(res.get("answer") or res["verdict"]).lower()
    answer_ok = (it["gold_answer_substring"].lower() in ans) if gold_fam == "DETERMINATE" else None
    rows.append({
        "id": iid, "domain": it["domain"], "stratum": it["stratum"], "extracted": True,
        "engine_verdict": res["verdict"], "engine_family": eng_fam,
        "gold_family": gold_fam, "verdict_match": verdict_match, "answer_ok": answer_ok,
        # auditability / localizability
        "byte_accounting_ok": res["byte_accounting_ok"],
        "fully_grounded": res["fully_grounded"],
        "assumptions": res["assumptions"],                # surfaced inferred-dispositive slots
        "hallucinated_rules": res["hallucinated_rules"],  # rule source_span not in policy
        "missing_slots": res.get("missing_slots_that_block", []),
        "n_slots": len(em["input_ir"].get("slots", {})), "n_rules": len(em["rulebook_ir"].get("rules", [])),
    })


def frac(p):
    n = sum(1 for r in rows if r.get("extracted"))
    return f"{sum(1 for r in rows if p(r))}/{n}" if n else "0/0"


agg = {
    "n_items": len(items),
    "extracted": frac(lambda r: r.get("extracted")),
    "byte_accounting_ok": frac(lambda r: r.get("byte_accounting_ok")),
    "no_hallucinated_rules": frac(lambda r: r.get("extracted") and not r.get("hallucinated_rules")),
    "verdict_family_match (informational)": frac(lambda r: r.get("verdict_match")),
    "underdetermined->INDETERMINATE (abstained, did not fabricate)":
        f"{sum(1 for r in rows if r.get('stratum') == 'underdetermined-baited' and r.get('engine_family') == 'INDETERMINATE')}"
        f"/{sum(1 for r in rows if r.get('stratum') == 'underdetermined-baited')}",
}

json.dump({"aggregate": agg, "rows": rows}, open(os.path.join(HERE, "adjud10_results.json"), "w"),
          ensure_ascii=False, indent=1)

print("\n=== ADJ101 10-item adjudication pilot (rule-engine arm) ===\n")
hdr = "{:<6} {:<22} {:>16} {:>14} {:>6} {:>6}".format("id", "stratum", "engine_verdict", "gold", "match", "audit")
print(hdr); print("-" * len(hdr))
for r in rows:
    if not r.get("extracted"):
        print("{:<6} NOT EXTRACTED".format(r["id"])); continue
    if r.get("engine_error"):
        print("{:<6} ENGINE ERROR: {}".format(r["id"], r["engine_error"][:60])); continue
    print("{:<6} {:<22} {:>16} {:>14} {:>6} {:>6}".format(
        r["id"], r["stratum"][:22], r["engine_family"], r["gold_family"],
        "Y" if r["verdict_match"] else "n", "Y" if r["byte_accounting_ok"] else "n"))
    notes = []
    if r["assumptions"]: notes.append(f"assumes={r['assumptions']}")
    if r["hallucinated_rules"]: notes.append(f"halluc_rules={r['hallucinated_rules']}")
    if r["missing_slots"]: notes.append(f"missing_slot={r['missing_slots']}")
    if notes: print("       -> " + "; ".join(notes))

print("\naggregate:", json.dumps(agg, indent=1))
