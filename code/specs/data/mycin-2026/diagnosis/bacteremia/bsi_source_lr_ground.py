#!/usr/bin/env python3
"""bsi_source_lr_ground.py — the write gate for the bacteremia PORTAL-OF-ENTRY LRs (G5b).

The source-id rulebook's strongest signal — which portal of entry → which bloodstream
organism — was authored "trust consensus". G5b grounds each source→organism ASSOCIATION
through the cold path (like the meningitis host factors, G2): the spider grounds the
direction + a verbatim byte-quote, an independent agent re-extracts, and this gate emits a
content-addressed bsi-source-lr-manifest.json (each association with verdict / trust /
byte-quote / the structural LR magnitude it carries). The LR MAGNITUDE stays structural —
the source grounds the direction, not an exact likelihood ratio (most series give marginals
and stratified tables, not a conditional LR — hence many records land direction_only).

Reuses the organism-id gate's gate/cite/safe_status. Like G5, this lands the grounded
provenance as a manifest + ledger artifact; regenerating source-id.adj's prior + LR values
from the two manifests is the G5c follow-up.

Usage:  python3 bsi_source_lr_ground.py [--check]
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
GROUNDING = MYCIN / "grounding" / "bsi-source-lr-grounding.json"
sys.path.insert(0, str(MYCIN / "diagnosis" / "organisms"))
import organism_id_ground as oid  # noqa: E402  (reuse: cite, gate, safe_status)

# The source→organism (and host→organism) associations, with the STRUCTURAL LR magnitude
# carried verbatim from source-id.adj. (grounding id, structural LR, evidence, organism)
SOURCE_LRS = [
    ("src_urinary_enteric", 14, "infection_source(urinary)", "enteric_gnb"),
    ("src_line_cons", 12, "infection_source(intravascular_line)", "coag_neg_staph"),
    ("src_line_saureus", 8, "infection_source(intravascular_line)", "s_aureus"),
    ("src_intraabd_enteric", 9, "infection_source(intraabdominal)", "enteric_gnb"),
    ("src_intraabd_anaerobes", 9, "infection_source(intraabdominal)", "anaerobes"),
    ("src_skin_saureus", 12, "infection_source(skin_soft_tissue)", "s_aureus"),
    ("src_skin_pyogenes", 6, "infection_source(skin_soft_tissue)", "strep_pyogenes"),
    ("src_resp_pneumo", 9, "infection_source(respiratory)", "s_pneumoniae"),
    ("host_neutropenia_pseudomonas", 6, "neutropenia(present)", "pseudomonas"),
    ("host_idu_saureus", 8, "injection_drug_use(present)", "s_aureus"),
]


def build(check: bool = False) -> int:
    if not GROUNDING.exists():
        print(f"bsi_source_lr_ground: {GROUNDING} not found — run the BSI source-LR spider first.",
              file=sys.stderr)
        return 2
    recs = {r["id"]: r for r in json.loads(GROUNDING.read_text())["records"]}
    clauses = {}
    for cid, lr, evidence, org in SOURCE_LRS:
        rec = recs.get(cid)
        status = rec["spider_status"] if rec else "missing"
        verdict, _ = oid.gate(status)
        # The association is grounded; the LR magnitude is structural → consensus when ACCEPT.
        trust = "consensus" if verdict == "ACCEPT" else "inferred"
        g = (rec or {}).get("grounded") or {}
        clauses[cid] = {
            "evidence": evidence, "organism": org, "lr": lr,
            "status": oid.safe_status(status), "verdict": verdict, "trust": trust,
            "byte_quote": g.get("byte_quote"), "url": g.get("resolved_url"),
        }
    accepted = sum(1 for c in clauses.values() if c["verdict"] == "ACCEPT")
    flagged = sum(1 for c in clauses.values() if c["verdict"] == "FLAG")
    manifest = {"kind": "bsi-source-lr", "clauses": clauses,
                "hash": hashlib.sha256(json.dumps(clauses, sort_keys=True).encode()).hexdigest()[:16]}

    if check:
        cur = HERE / "bsi-source-lr-manifest.json"
        ok = cur.exists() and json.loads(cur.read_text()).get("hash") == manifest["hash"]
        print("bsi_source_lr_ground --check:", "up to date" if ok else "OUT OF DATE")
        return 0 if ok else 1

    (HERE / "bsi-source-lr-manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"bsi_source_lr_ground: emitted bsi-source-lr-manifest.json ({accepted} ACCEPT grounded, "
          f"{flagged} FLAG). Run grounding/ground_sources.py to rebuild the provenance ledger.")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
