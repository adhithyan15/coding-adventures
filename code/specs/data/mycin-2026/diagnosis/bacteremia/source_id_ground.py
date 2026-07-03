#!/usr/bin/env python3
"""source_id_ground.py — regenerate the bacteremia source-id rulebook from the GROUNDED
manifests (G5c). The G5/G5b gates landed the grounded priors + portal-of-entry LRs as
manifests; this gate makes source-id.adj actually USE them — every clause whose value the
spider grounded now carries the grounded value + a byte-cited source + an upgraded trust
tier, while the clauses not yet grounded are carried verbatim, clearly marked (the same
honesty boundary the meningitis rulebook carries).

  bsi-prior-manifest.json (G5) + bsi-source-lr-manifest.json (G5b)  ──►  source-id.adj
        (priors)                    (source→organism + host LRs)         (regenerated)

STRUCTURE (which organisms/sources/clauses exist) is the closed vocabulary, fixed here;
VALUES + provenance + trust come from the manifests, never authored in this file. Reuses
the organism-id gate's cite/safe_status. Run grounding/ground_sources.py afterwards is not
needed (the manifests already feed the ledger); this gate only rewrites the .adj.

Usage:  python3 source_id_ground.py [--check]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
PRIOR_MANIFEST = HERE / "bsi-prior-manifest.json"
SRCLR_MANIFEST = HERE / "bsi-source-lr-manifest.json"
sys.path.insert(0, str(MYCIN / "diagnosis" / "organisms"))
import organism_id_ground as oid  # noqa: E402  (reuse: cite escaping, safe_status)

# Priors: (G5 grounding id | None, organism, fallback prior, authored source string).
PRIORS = [
    ("bsi_prior_saureus", "s_aureus", 0.22, "S. aureus a leading bloodstream isolate (standard microbiology)"),
    ("bsi_prior_enteric_gnb", "enteric_gnb", 0.25, "Enterobacterales the leading gram-negative bloodstream isolates"),
    ("bsi_prior_cons", "coag_neg_staph", 0.10, "CoNS common (often line-related / contaminant) (standard microbiology)"),
    ("bsi_prior_enterococcus", "enterococcus", 0.08, "Enterococcus a common nosocomial bloodstream isolate"),
    ("bsi_prior_spneumoniae", "s_pneumoniae", 0.07, "Pneumococcal bacteremia (community)"),
    ("bsi_prior_pseudomonas", "pseudomonas", 0.05, "Pseudomonas (healthcare-associated / neutropenic)"),
    ("bsi_prior_pyogenes", "strep_pyogenes", 0.04, "Group A strep bacteremia (skin/soft-tissue)"),
    (None, "anaerobes", 0.05, "Anaerobic bacteremia (intra-abdominal)"),
    ("bsi_prior_candida", "candida", 0.03, "Candidemia (line / gut translocation / neutropenia)"),
]
# Source→organism + host contributes: (G5b grounding id | None, lr, evidence, organism,
# authored source string). Order matches the rulebook's grouped layout.
CONTRIBS = [
    ("src_urinary_enteric", 14, "infection_source(urinary)", "enteric_gnb", "Urinary source → Enterobacterales (E. coli leading uropathogen)"),
    (None, 4, "infection_source(urinary)", "enterococcus", "Enterococcal urinary source (esp. healthcare-associated)"),
    (None, 2, "infection_source(urinary)", "pseudomonas", "Pseudomonas urinary source (catheter / healthcare)"),
    ("src_line_cons", 12, "infection_source(intravascular_line)", "coag_neg_staph", "Central-line-associated bloodstream infection: CoNS leading"),
    ("src_line_saureus", 8, "infection_source(intravascular_line)", "s_aureus", "CLABSI: S. aureus (IDSA intravascular catheter guidance)"),
    (None, 4, "infection_source(intravascular_line)", "candida", "Catheter-related candidemia (IDSA candidiasis guidance)"),
    ("src_intraabd_enteric", 9, "infection_source(intraabdominal)", "enteric_gnb", "Intra-abdominal source → enteric GNB (IDSA cIAI)"),
    ("src_intraabd_anaerobes", 9, "infection_source(intraabdominal)", "anaerobes", "Intra-abdominal source → anaerobes incl. B. fragilis (IDSA cIAI)"),
    (None, 4, "infection_source(intraabdominal)", "enterococcus", "Enterococcus in complicated intra-abdominal infection"),
    ("src_skin_saureus", 12, "infection_source(skin_soft_tissue)", "s_aureus", "Skin/soft-tissue source → S. aureus (IDSA SSTI)"),
    ("src_skin_pyogenes", 6, "infection_source(skin_soft_tissue)", "strep_pyogenes", "Skin/soft-tissue source → group A strep (IDSA SSTI)"),
    ("src_resp_pneumo", 9, "infection_source(respiratory)", "s_pneumoniae", "Pneumonia source → pneumococcal bacteremia"),
    (None, 3, "infection_source(respiratory)", "enteric_gnb", "Gram-negative pneumonia (Klebsiella) bacteremia"),
    ("host_neutropenia_pseudomonas", 6, "neutropenia(present)", "pseudomonas", "Febrile neutropenia → Pseudomonas (IDSA neutropenic fever)"),
    (None, 3, "neutropenia(present)", "enteric_gnb", "Febrile neutropenia → gram-negative bacteremia"),
    (None, 3, "neutropenia(present)", "candida", "Prolonged neutropenia → candidemia"),
    ("host_idu_saureus", 8, "injection_drug_use(present)", "s_aureus", "Injection drug use → S. aureus bacteremia / endocarditis"),
    (None, 6, "prosthetic_material(present)", "coag_neg_staph", "Prosthetic material → CoNS device infection"),
]

# Group headers between source blocks (by the evidence predicate's first appearance).
_GROUP_HEADER = {
    "infection_source(urinary)": "Source → organism (the strong signal)",
    "neutropenia(present)": "Host factors",
}


def _esc(s: str) -> str:
    """Escape a string for an adj-lang `source "..."` literal (backslash first, then quote,
    then collapse control chars) — the same discipline organism_id_ground.cite applies."""
    import re
    return re.sub(r"[\r\n\t]", " ", (s or "")[:90].replace("\\", "\\\\").replace('"', '\\"'))


def build(check: bool = False) -> int:
    if not (PRIOR_MANIFEST.exists() and SRCLR_MANIFEST.exists()):
        print("source_id_ground: BSI prior/source-LR manifests not found — run G5/G5b gates first.",
              file=sys.stderr)
        return 2
    priors = json.loads(PRIOR_MANIFEST.read_text())["clauses"]
    srclrs = json.loads(SRCLR_MANIFEST.read_text())["clauses"]

    lines = [
        "% ============================================================================",
        "% source-id — WHICH bacterium in the blood? The bacteremia organism-identification",
        "% rulebook, reasoning from the PORTAL OF ENTRY. Importable CAS library.",
        "% ============================================================================",
        "% GENERATED by source_id_ground.py from the GROUNDED manifests (G5 priors + G5b",
        "% portal-of-entry LRs). Do NOT hand-edit values — correct a wrong fact by editing",
        "% the grounding (re-run the spider) and re-running the gate. ACCEPTed clauses carry",
        "% the spider-grounded value + a byte-cited primary source; clauses not yet grounded",
        "% are carried verbatim at trust consensus/empirical and clearly marked.",
        "%",
        '% Use:  import "source-vocab.adj"  then add observe lines; the ? queries are here.',
        "",
        'import "source-vocab.adj"',
        "",
        "rulebook source_id {",
        "    use bacteremia_vocab",
        "",
        "    % ===================== Base priors (spider-grounded) =====================",
    ]
    manifest = {"kind": "source-id", "clauses": {}}

    def record(cid, verdict):
        manifest["clauses"][cid] = {"verdict": verdict}

    for gid, org, fallback, authored in PRIORS:
        c = priors.get(gid) if gid else None
        if c and c["verdict"] == "ACCEPT":
            lines += [f"    prior {c['value']} for {org}",
                      f'        source "{_esc(c.get("url") or "grounded")}"',
                      "        trust authoritative"]
            record(f"prior_{org}", "ACCEPT")
        else:
            tag = "   % [FLAG: " + oid.safe_status(c["status"]) + "]" if c else "   % [authored — pending grounding]"
            lines += [f"    prior {fallback} for {org}",
                      f'        source "{_esc(authored)}"',
                      f"        trust consensus{tag}"]
            record(f"prior_{org}", "FLAG" if c else "PENDING")

    prev_group = None
    for gid, lr, evidence, org, authored in CONTRIBS:
        if evidence in _GROUP_HEADER and _GROUP_HEADER[evidence] != prev_group:
            prev_group = _GROUP_HEADER[evidence]
            lines += ["", f"    % ===================== {prev_group} ====================="]
        c = srclrs.get(gid) if gid else None
        if c and c["verdict"] == "ACCEPT":
            lines += [f"    contributes {lr} from {evidence} to {org}",
                      f'        source "{_esc(c.get("url") or "grounded")}"',
                      "        trust consensus"]
            record(f"lr_{gid}", "ACCEPT")
        else:
            tag = "   % [FLAG: " + oid.safe_status(c["status"]) + "]" if c else "   % [authored — pending grounding]"
            lines += [f"    contributes {lr} from {evidence} to {org}",
                      f'        source "{_esc(authored)}"',
                      f"        trust empirical{tag}"]
            if gid:
                record(f"lr_{gid}", "FLAG" if c else "PENDING")

    lines += ["}", "",
              "% The differential: rank the competing bloodstream organisms by posterior."]
    for _, org, _, _ in PRIORS:
        lines.append(f"? {org}")
    adj_text = "\n".join(lines) + "\n"

    accepted = sum(1 for v in manifest["clauses"].values() if v["verdict"] == "ACCEPT")
    flagged = sum(1 for v in manifest["clauses"].values() if v["verdict"] == "FLAG")

    if check:
        cur = (HERE / "source-id.adj").read_text() if (HERE / "source-id.adj").exists() else ""
        ok = cur == adj_text
        print("source_id_ground --check:", "up to date" if ok else "OUT OF DATE")
        return 0 if ok else 1

    (HERE / "source-id.adj").write_text(adj_text)
    print(f"source_id_ground: regenerated source-id.adj from the grounded manifests "
          f"({accepted} ACCEPT grounded clauses, {flagged} flagged).")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
