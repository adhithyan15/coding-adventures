#!/usr/bin/env python3
"""ci_ground.py — the write gate for treatment CONTRAINDICATION / INTERACTION rules (CC-3).

CC-1/CC-2 turn chart facts into exclusions + dose-feasibility constraints; CC-3 grounds the
clinical RULES behind them through the same cold path. The spider
(grounding/workflows/ground-treatment-constraints.workflow.js) grounds each rule against a
primary source (FDA label / CDC / IDSA) with a verbatim byte-quote + an independent
adversarial re-extraction; this gate consumes that output
(grounding/treatment-constraints-grounding.json) and emits a content-addressed manifest of
the grounded rules — each carrying its verdict (ACCEPT/FLAG), trust, byte-quote and the
structural EFFECT it justifies in the optimizer. The constraint compiler's exclusions thus
rest on grounded facts, not authored ones.

The gate verdict (ACCEPT/FLAG) is REUSED from the organism-id gate — a treatment-constraint
rule is grounded knowledge pointed through the same machinery, like every other fact.

Usage:  python3 ci_ground.py [--check]
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
GROUNDING = MYCIN / "grounding" / "treatment-constraints-grounding.json"
sys.path.insert(0, str(MYCIN / "diagnosis" / "organisms"))
import organism_id_ground as oid  # noqa: E402  (reuse: gate, cite, safe_status)

# The structural EFFECT each grounded rule justifies in the constraint optimizer
# (chart_to_cop.py). Values come from the grounding; this is the closed list of rules.
EFFECTS = {
    "ci_penicillin_cephalosporin": "penicillin allergy → β-lactam class exclusion (3rd-gen cross-reactivity <1%)",
    "ci_aztreonam_safe_penicillin": "aztreonam usable WITH CAUTION in β-lactam allergy (cross-reactivity rare, not nil)",
    "ci_moxifloxacin_pregnancy": "moxifloxacin contraindicated in pregnancy → drug exclusion",
    "ci_tmpsmx_pregnancy": "tmp_smx contraindicated in pregnancy → drug exclusion",
    "ci_vancomycin_nephrotoxicity": "vancomycin nephrotoxicity → renal dose-ceiling penalty",
    "ci_aminoglycoside_vancomycin": "aminoglycoside + vancomycin → additive nephrotoxicity dose penalty",
    "ci_fluoroquinolone_qt": "fluoroquinolone QT prolongation → exclusion on QT prolongation",
    "ci_vancomycin_renal_dose": "vancomycin renal dose adjustment + level monitoring",
}


def build(check: bool = False) -> int:
    if not GROUNDING.exists():
        print(f"ci_ground: {GROUNDING} not found — run the treatment-constraints spider first.",
              file=sys.stderr)
        return 2
    recs = {r["id"]: r for r in json.loads(GROUNDING.read_text())["records"]}
    rules = {}
    for cid, effect in EFFECTS.items():
        rec = recs.get(cid)
        status = rec["spider_status"] if rec else "missing"
        verdict, trust = oid.gate(status)
        g = (rec or {}).get("grounded") or {}
        rules[cid] = {
            "effect": effect, "status": oid.safe_status(status), "verdict": verdict, "trust": trust,
            "byte_quote": g.get("byte_quote"), "url": g.get("resolved_url"),
            "source_title": (g.get("source_title") or "")[:90],
        }
    accepted = sum(1 for r in rules.values() if r["verdict"] == "ACCEPT")
    flagged = sum(1 for r in rules.values() if r["verdict"] == "FLAG")
    manifest = {"kind": "treatment-constraints", "rules": rules,
                "hash": hashlib.sha256(json.dumps(rules, sort_keys=True).encode()).hexdigest()[:16]}

    if check:
        cur = (HERE / "treatment-constraints.json")
        ok = cur.exists() and json.loads(cur.read_text()).get("hash") == manifest["hash"]
        print("ci_ground --check:", "up to date" if ok else "OUT OF DATE")
        return 0 if ok else 1

    (HERE / "treatment-constraints.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"ci_ground: emitted treatment-constraints.json ({accepted} ACCEPT grounded, "
          f"{flagged} FLAG). Run grounding/ground_sources.py to rebuild the provenance ledger.")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
