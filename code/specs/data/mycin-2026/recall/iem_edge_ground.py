#!/usr/bin/env python3
"""iem_edge_ground.py — the write gate for the inborn-errors-of-metabolism edges (REL-4).

The REL-1 IEM knowledge graph (`iem-edges.adj`) was authored "trust consensus" from
standard biochemistry — illustrative, NOT spider-grounded. REL-4 retires that
authored-debt through the cold path, exactly like the diagnosis/drug fronts (G1–G5):

    ground-iem-edges.workflow.js   (spider: WebSearch/WebFetch a PRIMARY source —
        OMIM / a biochemistry reference — per edge, verbatim byte-quote + verdict;
        an independent agent re-fetches and tries to refute)
            → recall/iem-edge-grounding.json
            → THIS GATE → regenerates iem-edges.adj + emits iem-edge-manifest.json

Each edge whose grounding the adversarial pass ACCEPTs is regenerated at
`trust authoritative` with the **grounded** byte-quote as its `source` (and the
fetched URL as its `locator`). Edges still pending / flagged / refuted keep the
authored `source` at `trust consensus`, tagged `% [FLAG: <status>]` so the debt is
visible. With no grounding JSON present, the gate regenerates the file BYTE-IDENTICALLY
to the authored seed (every edge consensus) — so `--check` is meaningful before the
spider has ever run, and the harness is testable without a live web pass.

Reuses the organism-id gate's gate / cite / safe_status (the shared grounding helpers).

Usage:  python3 iem_edge_ground.py            # regenerate iem-edges.adj + manifest
        python3 iem_edge_ground.py --check    # verify the .adj matches the manifest
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
ADJ = HERE / "iem-edges.adj"
GROUNDING = HERE / "iem-edge-grounding.json"
MANIFEST = HERE / "iem-edge-manifest.json"
sys.path.insert(0, str(MYCIN / "diagnosis" / "organisms"))
import organism_id_ground as oid  # noqa: E402  (reuse: gate, safe_status)


def _esc(s: str) -> str:
    """Escape an untrusted spider-derived string for an adj-lang `source "..."`
    literal — backslash FIRST (a trailing `\\` would escape the closing quote and
    corrupt the parse of following clauses), then the quote, then collapse control
    chars. Unlike `oid.cite` (which renders a 70-char source TITLE), this keeps the
    full byte-quote, which is what an edge's `source` carries."""
    s = s.replace("\\", "\\\\").replace('"', '\\"')
    return re.sub(r"[\r\n\t]", " ", s)

# The IEM knowledge graph, grouped by disease for the rendered `% ---` headers.
# (grounding id, relation, subject, object, AUTHORED fallback source). The authored
# source is what the seed carried; the spider replaces it with a grounded byte-quote
# when an edge is ACCEPTed. id = "<relation>__<subject>", stable across runs.
GROUPS: list[tuple[str, list[tuple[str, str, str, str]]]] = [
    ("Tay-Sachs (GM2 gangliosidosis)", [
        ("deficient_in", "tay_sachs", "hexosaminidase_a",
         "Tay-Sachs disease results from deficiency of the enzyme hexosaminidase A (HEXA)."),
        ("accumulates", "tay_sachs", "gm2_ganglioside",
         "GM2 ganglioside accumulates in neurons in Tay-Sachs disease."),
        ("inherited_as", "tay_sachs", "autosomal_recessive",
         "Tay-Sachs disease is inherited in an autosomal recessive manner."),
    ]),
    ("Gaucher disease", [
        ("deficient_in", "gaucher", "glucocerebrosidase",
         "Gaucher disease is caused by deficiency of glucocerebrosidase (acid beta-glucosidase)."),
        ("accumulates", "gaucher", "glucocerebroside",
         "Glucocerebroside accumulates in macrophages in Gaucher disease."),
        ("inherited_as", "gaucher", "autosomal_recessive",
         "Gaucher disease is inherited in an autosomal recessive manner."),
    ]),
    ("Phenylketonuria (PKU)", [
        ("deficient_in", "phenylketonuria", "phenylalanine_hydroxylase",
         "Phenylketonuria results from deficiency of phenylalanine hydroxylase."),
        ("accumulates", "phenylketonuria", "phenylalanine",
         "Phenylalanine accumulates in phenylketonuria."),
        ("inherited_as", "phenylketonuria", "autosomal_recessive",
         "Phenylketonuria is inherited in an autosomal recessive manner."),
    ]),
    ("Pompe disease (GSD II)", [
        ("deficient_in", "pompe", "acid_alpha_glucosidase",
         "Pompe disease is caused by deficiency of acid alpha-glucosidase (acid maltase)."),
        ("accumulates", "pompe", "lysosomal_glycogen",
         "Glycogen accumulates in lysosomes in Pompe disease."),
        ("inherited_as", "pompe", "autosomal_recessive",
         "Pompe disease is inherited in an autosomal recessive manner."),
    ]),
    ("Lesch-Nyhan syndrome", [
        ("deficient_in", "lesch_nyhan", "hgprt",
         "Lesch-Nyhan syndrome results from deficiency of hypoxanthine-guanine phosphoribosyltransferase (HGPRT)."),
        ("accumulates", "lesch_nyhan", "uric_acid",
         "Uric acid accumulates in Lesch-Nyhan syndrome."),
        ("inherited_as", "lesch_nyhan", "x_linked_recessive",
         "Lesch-Nyhan syndrome is inherited in an X-linked recessive manner."),
    ]),
    ("von Gierke disease (GSD I)", [
        ("deficient_in", "von_gierke", "glucose_6_phosphatase",
         "Von Gierke disease (glycogen storage disease type I) results from deficiency of glucose-6-phosphatase."),
        ("accumulates", "von_gierke", "glycogen",
         "Glycogen accumulates in liver and kidney in von Gierke disease."),
        ("inherited_as", "von_gierke", "autosomal_recessive",
         "Von Gierke disease is inherited in an autosomal recessive manner."),
    ]),
]

HEADER = """\
% ============================================================================
% iem-edges — the inborn-errors-of-metabolism knowledge-graph (REL-1 seed, REL-4 grounded).
% ============================================================================
% MYCIN-2026. A typed graph of DISEASE -> ENZYME / SUBSTRATE / INHERITANCE edges,
% the densest, cleanest fact-recall family in medicine. "Which enzyme is deficient
% in Tay-Sachs?" becomes the binding query  ? deficient_in(tay_sachs, $Enzyme).
%
% GENERATED by recall/iem_edge_ground.py from recall/iem-edge-grounding.json (the
% spider's byte-provenanced output). Do not hand-edit — re-ground and regenerate.
% An ACCEPTed edge carries its grounded byte-quote as `source` + `trust authoritative`
% (+ the fetched URL as `locator`); a still-ungrounded edge keeps the authored source
% at `trust consensus`, tagged `% [FLAG: <status>]`, so the authored-debt is visible
% and drives to zero. (See feedback_nothing_human_authored.)
% ============================================================================

dictionary biochem_iem {
    define disease      : entity   surface "inborn error of metabolism", "metabolic disease"
    define enzyme       : entity   surface "enzyme", "enzyme deficiency"
    define substrate    : entity   surface "accumulated substrate", "stored material"
    define inheritance  : entity   surface "inheritance pattern"

    define deficient_in : relation from disease to enzyme
    define accumulates  : relation from disease to substrate
    define inherited_as : relation from disease to inheritance
}
"""


def _edge_block(rel: str, subj: str, obj: str, authored: str, rec: dict | None) -> tuple[str, dict]:
    """Render one `relate` clause + return its manifest entry. A grounded record
    (ACCEPT) supplies the byte-quote/url and lifts trust to authoritative; otherwise
    the authored source stays at consensus with a FLAG tag."""
    status = rec["spider_status"] if rec else "pending"
    verdict, _ = oid.gate(status)
    g = (rec or {}).get("grounded") or {}
    if verdict == "ACCEPT":
        source = g.get("byte_quote") or authored
        trust = "authoritative"
        url = g.get("resolved_url")
        lines = [f"    relate {rel}({subj}, {obj})",
                 f'        source "{_esc(source)}"']
        if url:
            lines.append(f'        locator "{_esc(url)}"')
        lines.append("        trust authoritative")
    else:
        source = authored
        trust = "consensus"
        url = None
        lines = [f"    relate {rel}({subj}, {obj})",
                 f'        source "{_esc(authored)}"',
                 f"        trust consensus   % [FLAG: {oid.safe_status(status)}]"]
    entry = {"relation": rel, "subject": subj, "object": obj,
             "status": oid.safe_status(status), "verdict": verdict,
             "trust": trust, "source": source, "url": url}
    return "\n".join(lines), entry


def build(check: bool = False) -> int:
    recs = {}
    if GROUNDING.exists():
        recs = {r["id"]: r for r in json.loads(GROUNDING.read_text()).get("records", [])}

    body = [HEADER, "rulebook iem_facts {", "    use biochem_iem"]
    clauses: dict[str, dict] = {}
    for label, edges in GROUPS:
        body.append("")
        body.append(f"    % --- {label} " + "-" * max(0, 64 - len(label)))
        for rel, subj, obj, authored in edges:
            eid = f"{rel}__{subj}"
            block, entry = _edge_block(rel, subj, obj, authored, recs.get(eid))
            body.append(block)
            clauses[eid] = entry
    body.append("}")
    adj_text = "\n".join(body) + "\n"

    accepted = sum(1 for c in clauses.values() if c["verdict"] == "ACCEPT")
    flagged = sum(1 for c in clauses.values() if c["verdict"] != "ACCEPT")
    manifest = {"kind": "iem-edge", "clauses": clauses,
                "hash": hashlib.sha256(json.dumps(clauses, sort_keys=True).encode()).hexdigest()[:16]}

    if check:
        ok = ADJ.exists() and ADJ.read_text() == adj_text
        mok = MANIFEST.exists() and json.loads(MANIFEST.read_text()).get("hash") == manifest["hash"]
        print("iem_edge_ground --check:", "up to date" if (ok and mok) else "OUT OF DATE")
        return 0 if (ok and mok) else 1

    ADJ.write_text(adj_text)
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"iem_edge_ground: regenerated iem-edges.adj + iem-edge-manifest.json "
          f"({accepted} ACCEPT grounded, {flagged} consensus/flagged authored-debt). "
          f"Run grounding/ground_sources.py to rebuild the provenance ledger.")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
