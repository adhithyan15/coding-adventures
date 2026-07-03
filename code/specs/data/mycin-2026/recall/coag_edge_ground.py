#!/usr/bin/env python3
"""coag_edge_ground.py — the write gate for the coagulation edges (REL-13).

A FIFTH recall domain — coagulation / bleeding disorders (hematology). Same machinery
as the IEM / vitamin / anemia / endocrine gates: it REUSES iem_edge_ground._edge_block
(the pure renderer) and the shared organism_id_ground helpers. Only the vocabulary and
edges differ. Three board-classic relations let a single disorder be queried three ways:

    factor_deficiency(disorder, clotting_factor)   hemophilia_a → factor_viii, …
    coag_inheritance(disorder, inheritance_pattern) hemophilia_a → x_linked_recessive, …
    prolonged_test(disorder, screening_test)        hemophilia_a → aptt, …

Owns coag-edges.adj. A grounded edge lifts to `trust authoritative` with its byte-quote
+ URL; an ungrounded edge stays `trust consensus` + `% [FLAG: …]`. With no grounding
JSON it regenerates byte-identically (so --check is a stable CI gate).

Usage:  python3 coag_edge_ground.py            # regenerate coag-edges.adj + manifest
        python3 coag_edge_ground.py --check    # verify the .adj matches the manifest
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "coag-edges.adj"
GROUNDING = HERE / "coag-edge-grounding.json"
MANIFEST = HERE / "coag-edge-manifest.json"
sys.path.insert(0, str(HERE))
import iem_edge_ground as iem  # noqa: E402  (reuse the pure _edge_block renderer)

# The coagulation knowledge graph, grouped by disorder for the rendered `% ---` headers.
# (relation, subject, object, AUTHORED fallback source). Board-classic hematology:
# the intrinsic-pathway factor deficiencies (VIII/IX/XI) prolong the aPTT and the
# extrinsic-pathway deficiency (VII) prolongs the PT; vWD classically prolongs the
# bleeding time (defective platelet adhesion), with VIII carried/stabilized by vWF.
GROUPS: list[tuple[str, list[tuple[str, str, str, str]]]] = [
    ("Hemophilia A (factor VIII deficiency)", [
        ("factor_deficiency", "hemophilia_a", "factor_viii",
         "Hemophilia A is caused by a deficiency of coagulation factor VIII."),
        ("coag_inheritance", "hemophilia_a", "x_linked_recessive",
         "Hemophilia A is inherited in an X-linked recessive pattern."),
        ("prolonged_test", "hemophilia_a", "aptt",
         "Factor VIII deficiency prolongs the activated partial thromboplastin time (aPTT) "
         "with a normal prothrombin time."),
    ]),
    ("Hemophilia B (Christmas disease, factor IX deficiency)", [
        ("factor_deficiency", "hemophilia_b", "factor_ix",
         "Hemophilia B (Christmas disease) is caused by a deficiency of coagulation factor IX."),
        ("coag_inheritance", "hemophilia_b", "x_linked_recessive",
         "Hemophilia B is inherited in an X-linked recessive pattern."),
        ("prolonged_test", "hemophilia_b", "aptt",
         "Factor IX deficiency prolongs the activated partial thromboplastin time (aPTT) "
         "with a normal prothrombin time."),
    ]),
    ("Von Willebrand disease", [
        ("factor_deficiency", "von_willebrand_disease", "von_willebrand_factor",
         "Von Willebrand disease results from a quantitative or qualitative deficiency of "
         "von Willebrand factor."),
        ("coag_inheritance", "von_willebrand_disease", "autosomal_dominant",
         "Von Willebrand disease (the common type 1) is inherited in an autosomal dominant pattern."),
        ("prolonged_test", "von_willebrand_disease", "bleeding_time",
         "Von Willebrand disease classically prolongs the bleeding time because von Willebrand "
         "factor mediates platelet adhesion."),
    ]),
    ("Factor VII deficiency", [
        ("factor_deficiency", "factor_vii_deficiency", "factor_vii",
         "Factor VII deficiency is a deficiency of coagulation factor VII of the extrinsic pathway."),
        ("coag_inheritance", "factor_vii_deficiency", "autosomal_recessive",
         "Factor VII deficiency is inherited in an autosomal recessive pattern."),
        ("prolonged_test", "factor_vii_deficiency", "pt",
         "Factor VII deficiency prolongs the prothrombin time (PT) in isolation, with a "
         "normal activated partial thromboplastin time."),
    ]),
    ("Hemophilia C (factor XI deficiency)", [
        ("factor_deficiency", "hemophilia_c", "factor_xi",
         "Hemophilia C is caused by a deficiency of coagulation factor XI."),
        ("coag_inheritance", "hemophilia_c", "autosomal_recessive",
         "Hemophilia C (factor XI deficiency) is inherited in an autosomal recessive pattern."),
        ("prolonged_test", "hemophilia_c", "aptt",
         "Factor XI deficiency prolongs the activated partial thromboplastin time (aPTT)."),
    ]),
]

HEADER = """\
% ============================================================================
% coag-edges — the coagulation / bleeding-disorder knowledge-graph (MYCIN-2026 REL-13).
% ============================================================================
% A FIFTH recall domain — coagulation (hematology). "Which factor is deficient in
% hemophilia B?" becomes the binding query  ? factor_deficiency(hemophilia_b, $Factor).
% Three relations query one disorder three ways: deficient factor, inheritance pattern,
% and the screening test it prolongs (intrinsic VIII/IX/XI → aPTT; extrinsic VII → PT;
% vWD → bleeding time).
%
% GENERATED by recall/coag_edge_ground.py from recall/coag-edge-grounding.json
% (the spider's byte-provenanced output). Do not hand-edit — re-ground and regenerate.
% A grounded edge carries its byte-quote as `source` + `trust authoritative` (+ URL as
% `locator`); an ungrounded edge stays `trust consensus`, tagged `% [FLAG: <status>]`,
% so the authored-debt is visible and drives to zero. (See feedback_nothing_human_authored.)
% ============================================================================

dictionary coag_vocab {
    define disorder    : entity   surface "bleeding disorder", "coagulopathy"
    define factor      : entity   surface "clotting factor", "coagulation factor"
    define pattern     : entity   surface "inheritance pattern"
    define test        : entity   surface "screening test", "coagulation assay"

    define factor_deficiency : relation from disorder to factor
    define coag_inheritance  : relation from disorder to pattern
    define prolonged_test    : relation from disorder to test
}
"""


def build(check: bool = False) -> int:
    recs = {}
    if GROUNDING.exists():
        recs = {r["id"]: r for r in json.loads(GROUNDING.read_text()).get("records", [])}

    body = [HEADER, "rulebook coag_facts {", "    use coag_vocab"]
    clauses: dict[str, dict] = {}
    for label, edges in GROUPS:
        body.append("")
        body.append(f"    % --- {label} " + "-" * max(0, 64 - len(label)))
        for rel, subj, obj, authored in edges:
            eid = f"{rel}__{subj}"
            block, entry = iem._edge_block(rel, subj, obj, authored, recs.get(eid))
            body.append(block)
            clauses[eid] = entry
    body.append("}")
    adj_text = "\n".join(body) + "\n"

    accepted = sum(1 for c in clauses.values() if c["verdict"] == "ACCEPT")
    flagged = sum(1 for c in clauses.values() if c["verdict"] != "ACCEPT")
    manifest = {"kind": "coag-edge", "clauses": clauses,
                "hash": hashlib.sha256(json.dumps(clauses, sort_keys=True).encode()).hexdigest()[:16]}

    if check:
        ok = ADJ.exists() and ADJ.read_text() == adj_text
        mok = MANIFEST.exists() and json.loads(MANIFEST.read_text()).get("hash") == manifest["hash"]
        print("coag_edge_ground --check:", "up to date" if (ok and mok) else "OUT OF DATE")
        return 0 if (ok and mok) else 1

    ADJ.write_text(adj_text)
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"coag_edge_ground: regenerated coag-edges.adj + coag-edge-manifest.json "
          f"({accepted} ACCEPT grounded, {flagged} consensus/flagged authored-debt). "
          f"Run grounding/ground_sources.py to rebuild the provenance ledger.")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
