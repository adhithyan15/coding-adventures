#!/usr/bin/env python3
"""bsi_prior_ground.py — the write gate for the bacteremia/BSI organism PRIORS (G5).

The bacteremia source-id rulebook's base priors were authored "trust consensus". This gate
consumes the spider output (grounding/bsi-prior-grounding.json: a primary-source byte-quote
+ adversarial verdict per organism) and emits a content-addressed bsi-prior-manifest.json —
each prior with its grounded proportion, verdict (ACCEPT/FLAG), trust, byte-quote and source.
It REUSES the organism-id gate's parse_proportion / cite / gate / safe_status (a new domain
is grounded FACTS through the same machinery, not new code).

Like G3 (doses), this lands the grounded provenance as a manifest + a system-ledger artifact;
regenerating source-id.adj's prior values from the manifest is the G5b follow-up.

Usage:  python3 bsi_prior_ground.py [--check]
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
GROUNDING = MYCIN / "grounding" / "bsi-prior-grounding.json"
sys.path.insert(0, str(MYCIN / "diagnosis" / "organisms"))
import organism_id_ground as oid  # noqa: E402  (reuse: parse_proportion, cite, gate, safe_status)

# The bloodstream organisms the spider grounded (grounding id → organism), plus a fallback
# prior used only if a record is ungrounded. The closed list of priors this gate grades.
PRIORS = [
    ("bsi_prior_saureus", "s_aureus", 0.22),
    ("bsi_prior_enteric_gnb", "enteric_gnb", 0.25),
    ("bsi_prior_cons", "coag_neg_staph", 0.10),
    ("bsi_prior_enterococcus", "enterococcus", 0.08),
    ("bsi_prior_spneumoniae", "s_pneumoniae", 0.07),
    ("bsi_prior_pseudomonas", "pseudomonas", 0.05),
    ("bsi_prior_pyogenes", "strep_pyogenes", 0.04),
    ("bsi_prior_candida", "candida", 0.03),
]


def build(check: bool = False) -> int:
    if not GROUNDING.exists():
        print(f"bsi_prior_ground: {GROUNDING} not found — run the BSI-prior spider first.",
              file=sys.stderr)
        return 2
    recs = {r["id"]: r for r in json.loads(GROUNDING.read_text())["records"]}
    clauses = {}
    for cid, org, fallback in PRIORS:
        rec = recs.get(cid)
        status = rec["spider_status"] if rec else "missing"
        verdict, trust = oid.gate(status)
        grounded_val = oid.parse_proportion((rec or {}).get("grounded", {}).get("value_found", "")) if rec else None
        value = grounded_val if (verdict == "ACCEPT" and grounded_val is not None) else fallback
        g = (rec or {}).get("grounded") or {}
        clauses[cid] = {
            "organism": org, "status": oid.safe_status(status), "verdict": verdict, "trust": trust,
            "value": value, "fallback": fallback,
            "byte_quote": g.get("byte_quote"), "url": g.get("resolved_url"),
        }
    accepted = sum(1 for c in clauses.values() if c["verdict"] == "ACCEPT")
    flagged = sum(1 for c in clauses.values() if c["verdict"] == "FLAG")
    manifest = {"kind": "bsi-prior", "clauses": clauses,
                "hash": hashlib.sha256(json.dumps(clauses, sort_keys=True).encode()).hexdigest()[:16]}

    if check:
        cur = HERE / "bsi-prior-manifest.json"
        ok = cur.exists() and json.loads(cur.read_text()).get("hash") == manifest["hash"]
        print("bsi_prior_ground --check:", "up to date" if ok else "OUT OF DATE")
        return 0 if ok else 1

    (HERE / "bsi-prior-manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"bsi_prior_ground: emitted bsi-prior-manifest.json ({accepted} ACCEPT grounded, "
          f"{flagged} FLAG). Run grounding/ground_sources.py to rebuild the provenance ledger.")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
