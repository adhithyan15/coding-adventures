#!/usr/bin/env python3
"""organism_id_ground.py — the adversarial WRITE GATE for organism identification.

Retiring authoring debt: the organism-id rulebook's epidemiologic priors and
gram-stain morphology mappings used to be HAND-AUTHORED (values typed from memory
of Brouwer/IDSA). This gate replaces that — it consumes the SPIDER's output
(grounding/organism-id-grounding.json: a primary-source byte-quote + independent
re-extraction + adversarial verdict per claim) and regenerates organism-id.adj so
every gradeable clause is GROUNDED, byte-cited, and gated, never authored.

The gate (mirrors cas_build.py's clause gate):
  - spider_status `grounded` (a primary source affirms it AND the byte-quote was
    re-extraction-stable)         → ACCEPT: the value comes from the source; trust
                                    `authoritative`; the byte-quote + URL is the source.
  - `direction_only` (the source supports the direction but the magnitude wasn't
    byte-anchored / re-extraction unstable) → FLAG: kept at trust `inferred`, cited.
  - `refuted` (a source contradicts it)     → the authored value is wrong; FLAGGED with
                                    the refutation (a prior is structural, never deleted).
  - missing / `ungrounded`                  → FLAG: carried at trust `inferred`, marked
                                    pending grounding (still authoring debt).

Host-factor contributes are NOT in this grounding batch yet, so they are carried
verbatim but tagged pending-grounding (debt) — the provenance ledger tracks them.

Usage:  python3 organism_id_ground.py [--check]
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
GROUNDING = MYCIN / "grounding" / "organism-id-grounding.json"
HOST_GROUNDING = MYCIN / "grounding" / "host-factor-grounding.json"

# The rulebook STRUCTURE (closed vocabulary — which organisms / findings exist).
# VALUES + provenance + trust come from the grounding, never authored here.
PRIORS = [  # (grounding id, organism, fallback prior if ungrounded)
    ("prior_s_pneumoniae", "s_pneumoniae", 0.50),
    ("prior_n_meningitidis", "n_meningitidis", 0.15),
    ("prior_h_influenzae", "h_influenzae", 0.05),
    ("prior_listeria", "listeria", 0.05),
    ("prior_gram_negative_bacilli", "gram_negative_bacilli", 0.03),
    ("prior_group_b_strep", "group_b_strep", 0.02),
    ("prior_s_aureus", "s_aureus", 0.01),
]
MORPH = [  # (grounding id, morphology value, organism, definitional LR magnitude)
    ("morph_gpd_pneumococcus", "gram_positive_diplococci", "s_pneumoniae", 50),
    ("morph_gnd_meningococcus", "gram_negative_diplococci", "n_meningitidis", 50),
    ("morph_gpb_listeria", "gram_positive_bacilli", "listeria", 45),
    ("morph_gncocco_hflu", "gram_negative_coccobacilli", "h_influenzae", 45),
    ("morph_gnb_enteric", "gram_negative_bacilli", "gram_negative_bacilli", 40),
    ("morph_gpcc_saureus", "gram_positive_cocci_clusters", "s_aureus", 45),
]
# Host-factor contributes (G2): the host-factor → organism ASSOCIATION is spider-
# grounded (grounding/host-factor-grounding.json); the LR MAGNITUDE stays a structural
# risk-multiplier (like the morphology LRs — the source grounds direction, not a number).
# (grounding id, structural LR, evidence predicate, organism)
HOST = [
    ("host_neonate_gbs", 12, "age_band(neonate)", "group_b_strep"),
    ("host_neonate_gnb", 8, "age_band(neonate)", "gram_negative_bacilli"),
    ("host_neonate_listeria", 4, "age_band(neonate)", "listeria"),
    ("host_olderadult_listeria", 5, "age_band(older_adult)", "listeria"),
    ("host_olderadult_gnb", 2, "age_band(older_adult)", "gram_negative_bacilli"),
    ("host_infantchild_nmen", 2, "age_band(infant_child)", "n_meningitidis"),
    ("host_infantchild_hflu", 2, "age_band(infant_child)", "h_influenzae"),
    ("host_immuno_listeria", 6, "immunocompromised(present)", "listeria"),
    ("host_immuno_gnb", 3, "immunocompromised(present)", "gram_negative_bacilli"),
    ("host_listeriaexp_listeria", 9, "listeria_exposure(present)", "listeria"),
    ("host_neurosurg_saureus", 8, "recent_neurosurgery_or_shunt(present)", "s_aureus"),
    ("host_neurosurg_gnb", 6, "recent_neurosurgery_or_shunt(present)", "gram_negative_bacilli"),
    ("host_crowding_nmen", 4, "crowding_exposure(present)", "n_meningitidis"),
    ("host_petechial_nmen", 7, "petechial_rash(present)", "n_meningitidis"),
]

PROP_RE = re.compile(r"(?:proportion[^0-9]*~?|~)\s*(0?\.\d+)")
PCT_RE = re.compile(r"(\d+(?:\.\d+)?)\s*%")


def parse_proportion(value_found: str) -> float | None:
    """Pull the source-derived proportion (0–1) out of the spider's value text. The
    text is web/LLM-derived (untrusted), so an out-of-range value (e.g. "999%") is
    rejected (→ None → the fallback prior) — a prior must be a probability."""
    # Bound the (spider/web-derived, untrusted) input before regex search — a very long
    # crafted string could make PCT_RE/PROP_RE superlinear (ReDoS). A real proportion
    # phrase is short; 200 chars is ample.
    value_found = (value_found or "")[:200]
    m = PROP_RE.search(value_found)
    if m:
        v = float(m.group(1))
    else:
        m = PCT_RE.search(value_found)
        if not m:
            return None
        v = round(float(m.group(1)) / 100.0, 4)
    return v if 0.0 <= v <= 1.0 else None


KNOWN_STATUS = {"grounded", "direction_only", "refuted", "ungrounded", "missing"}


def safe_status(status: object) -> str:
    """The spider_status is semi-trusted (spider/web/LLM-derived) and is interpolated into
    a `% [FLAG: <status>]` COMMENT in the generated rulebook. An adj-lang line comment ends
    at a newline, so a crafted status containing a newline could close the comment and inject
    a live clause. Coerce anything outside the known vocabulary to a safe literal (strip to
    [a-z_], bounded) so no untrusted bytes reach the generated program — the same discipline
    cite() applies to source titles."""
    s = str(status)
    if s in KNOWN_STATUS:
        return s
    cleaned = re.sub(r"[^a-z_]", "", s.lower())[:32]
    return cleaned or "ungrounded"


def gate(status: str) -> tuple[str, str]:
    """(verdict, trust) for a gradeable clause given its spider_status."""
    if status == "grounded":
        return "ACCEPT", "authoritative"
    if status == "refuted":
        return "FLAG", "inferred"   # the authored value is wrong; kept+flagged, never silently used
    if status in ("direction_only", "ungrounded"):
        return "FLAG", "inferred"
    return "FLAG", "inferred"       # missing record


def cite(rec: dict | None) -> str:
    if not rec:
        return "authored hint — PENDING GROUNDING"
    g = rec.get("grounded") or {}
    title = (g.get("source_title") or g.get("resolved_url") or "grounded")[:70]
    # The citation is spider/web-derived (untrusted) and is emitted into a `source
    # "..."` adj-lang string literal. Escape for that grammar — backslash FIRST (an
    # unescaped trailing `\` would escape the closing quote and corrupt the parse of
    # following clauses), then the quote, then collapse control chars.
    title = title.replace("\\", "\\\\").replace('"', '\\"')
    return re.sub(r"[\r\n\t]", " ", title)


def build(check: bool = False) -> int:
    if not GROUNDING.exists():
        print(f"organism_id_ground: {GROUNDING} not found — run the organism-id spider first.",
              file=sys.stderr)
        return 2
    recs = {r["id"]: r for r in json.loads(GROUNDING.read_text())["records"]}
    # Host-factor grounding (G2) — optional; if absent, host clauses stay authored-debt.
    host_recs: dict[str, dict] = {}
    if HOST_GROUNDING.exists():
        host_recs = {r["id"]: r for r in json.loads(HOST_GROUNDING.read_text())["records"]}

    lines = [
        "% ============================================================================",
        "% organism-id — GROUNDED organism-identification rulebook (spider + write gate).",
        "% ============================================================================",
        "% GENERATED by organism_id_ground.py from grounding/organism-id-grounding.json.",
        "% Do NOT hand-edit values — correct a wrong fact by editing the grounding (the",
        "% spider's record) and re-running the gate. Every prior/morphology clause below",
        "% is byte-cited to a primary source; ACCEPTed clauses carry the source value,",
        "% FLAGged clauses are kept at trust inferred (direction grounded / pending).",
        "",
        'import "organism-vocab.adj"',
        "",
        "rulebook organism_id {",
        "    use organism_vocab",
        "",
        "    % ===================== Epidemiologic priors (spider-grounded) =====================",
    ]
    manifest = {"kind": "organism-id", "clauses": {}}

    def record_clause(cid, status, verdict, trust, value, rec):
        manifest["clauses"][cid] = {
            "status": status, "verdict": verdict, "trust": trust, "value": value,
            "byte_quote": ((rec or {}).get("grounded") or {}).get("byte_quote"),
            "url": ((rec or {}).get("grounded") or {}).get("resolved_url"),
        }

    for cid, org, fallback in PRIORS:
        rec = recs.get(cid)
        status = rec["spider_status"] if rec else "missing"
        verdict, trust = gate(status)
        grounded_val = parse_proportion((rec or {}).get("grounded", {}).get("value_found", "")) if rec else None
        value = grounded_val if (verdict == "ACCEPT" and grounded_val is not None) else fallback
        tag = "" if verdict == "ACCEPT" else "   % [FLAG: " + safe_status(status) + "]"
        lines += [f"    prior {value} for {org}",
                  f'        source "{cite(rec)}"',
                  f"        trust {trust}{tag}"]
        record_clause(cid, status, verdict, trust, value, rec)

    lines += ["", "    % ===================== Gram-stain morphology (spider-grounded mapping) ====================="]
    for cid, morph, org, lr in MORPH:
        rec = recs.get(cid)
        status = rec["spider_status"] if rec else "missing"
        verdict, trust = gate(status)
        # The mapping is what the spider grounds; the LR magnitude is a definitional
        # near-certainty (like csf_culture in the meningitis arm), so trust consensus
        # when the mapping is grounded.
        trust = "consensus" if verdict == "ACCEPT" else "inferred"
        tag = "" if verdict == "ACCEPT" else "   % [FLAG: " + safe_status(status) + "]"
        lines += [f"    contributes {lr} from csf_gram_morphology({morph}) to {org}",
                  f'        source "{cite(rec)}"',
                  f"        trust {trust}{tag}"]
        record_clause(cid, status, verdict, trust, lr, rec)

    lines += ["", "    % ===================== Host factors (spider-grounded associations) ====================="]
    pending_host = 0
    for cid, lr, evidence, org in HOST:
        rec = host_recs.get(cid)
        if rec is None:
            # No grounding record yet → still authoring debt (carried, marked pending).
            pending_host += 1
            lines += [f"    contributes {lr} from {evidence} to {org}",
                      '        source "authored — PENDING GROUNDING (provenance ledger)"',
                      "        trust inferred"]
            manifest["clauses"][cid] = {"status": "missing", "verdict": "PENDING",
                                        "trust": "inferred", "value": lr,
                                        "byte_quote": None, "url": None}
            continue
        status = rec["spider_status"]
        verdict, _ = gate(status)
        # The ASSOCIATION is grounded; the LR magnitude is structural (a risk-multiplier),
        # so a grounded host factor is consensus-level — like the morphology mappings.
        trust = "consensus" if verdict == "ACCEPT" else "inferred"
        tag = "" if verdict == "ACCEPT" else "   % [FLAG: " + safe_status(status) + "]"
        lines += [f"    contributes {lr} from {evidence} to {org}",
                  f'        source "{cite(rec)}"',
                  f"        trust {trust}{tag}"]
        record_clause(cid, status, verdict, trust, lr, rec)

    lines += ["}", ""]
    for org in [o for _, o, _ in PRIORS]:
        lines.append(f"? {org}")
    adj_text = "\n".join(lines) + "\n"
    digest = hashlib.sha256(adj_text.encode()).hexdigest()[:16]
    manifest["hash"] = digest

    accepted = sum(1 for c in manifest["clauses"].values() if c["verdict"] == "ACCEPT")
    flagged = sum(1 for c in manifest["clauses"].values() if c["verdict"] == "FLAG")

    if check:
        cur = (HERE / "organism-id.adj").read_text() if (HERE / "organism-id.adj").exists() else ""
        ok = cur == adj_text
        print("organism_id_ground --check:", "up to date" if ok else "OUT OF DATE")
        return 0 if ok else 1

    (HERE / "organism-id.adj").write_text(adj_text)
    (HERE / "organism-id-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    # The system-wide PROVENANCE-LEDGER.md is owned by grounding/ground_sources.py, which
    # reads this manifest (counts + citation verification). We only emit .adj + manifest.
    print(f"organism_id_ground: regenerated organism-id.adj from grounding "
          f"({accepted} ACCEPTED grounded, {flagged} FLAGGED; {pending_host} host clauses "
          f"pending). Run grounding/ground_sources.py to rebuild the provenance ledger.")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
