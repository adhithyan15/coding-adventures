#!/usr/bin/env python3
"""dose_caps_build.py — generate the CONJUNCTIVE DOSE-CAP RULEBOOK as ADJ `rule`s.

WHY THIS EXISTS (the ADJ-native refactor, CC-2b).  The hepatorenal dose cap first landed as
a Python post-loop block inside `chart_to_cop.py`:

    has_hepatic = any(r.startswith("hepatic_") for r in cop.risks)
    has_renal   = any(r.startswith("renal_")   for r in cop.risks)
    if has_hepatic and has_renal:                 # <- the CONJUNCTION, reasoned in Python
        cop.risks.add("hepatorenal")

That is exactly the "Python rule layer in the middle" we are removing (the same move that
turned `_PREGNANCY_CONTRAINDICATED` into `contraindications.adj`).  A dose cap that fires on a
CONJUNCTION of patient risk factors is *conditional knowledge* — "ceftriaxone is capped WHEN
hepatic impairment AND significant renal impairment co-occur" — precisely what an adj-lang
`rule { head: … when: … }` clause expresses.  So the conjunction moves OUT of Python and INTO
an ADJ rulebook the engine reasons over; Python is left only asserting the patient's raw
active risk tokens and reading back the engine's DERIVED compound risk + grounded cap.

    grounding/dose-window-grounding.json   (record dose_cap_ceftriaxone_hepatorenal, grounded)
            │
            ▼  THIS GENERATOR
    dose_caps.adj          a rulebook of definitional category/compound facts, a GROUNDED
                           `dose_capped_under` fact (the FDA byte-quote), and TWO generic
                           conjunction rules
            │
            ▼  dose_caps.derive_dose_caps(cli, active_risks)
    ? derived_risk($R) / ? dose_capped($D, $R)   the ENGINE derives the compound risk and the
                                                 capped drug (0 model calls)

THE GENERIC SHAPE (this is the point — not hepatorenal-specific).  A compound risk is "factor
C holds when category A AND category B are both present"; a dose cap is "drug D is capped under
risk C".  Both are expressed as plain binary relations + two STRUCTURAL rules, so ANY
two-factor compound risk (and any drug capped under it) is added as data, with no new Python
branch and no new rule.  `risk_in_category` lets a chart assert a *graded* token
(`hepatic_severe`) and still match the category-level rule — the severity→category mapping is
itself in the language, not the compiler.

GROUNDING / TRUST.  The `dose_capped_under` fact carries the spider's verbatim byte-quote as
`source` + the fetched URL as `locator`.  A dose cap is DIRECTIONAL (a ceiling exists / the
dose must shrink — not a magnitude; the exact mg figure is the formulary's illustrative
feasibility model), so a grounding whose verdict confirms the cap's existence + direction
(`grounded` or `direction_only`) WITH a byte-quote grounds it at `trust authoritative`; any
other state stays `trust consensus`, tagged `% [FLAG: …]` so the authored-debt is visible and
drives to zero.  With no grounding JSON present the file regenerates byte-identically from the
authored fallbacks, so `--check` is meaningful offline and the harness is testable without a
web pass.

Usage:  python3 dose_caps_build.py            # regenerate .adj + manifest
        python3 dose_caps_build.py --check    # verify the .adj matches the manifest
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parents[1]  # .../mycin-2026 (antibiotics → treatment → mycin-2026)
ADJ = HERE / "dose_caps.adj"
GROUNDING = MYCIN / "grounding" / "dose-window-grounding.json"
MANIFEST = HERE / "dose-cap-manifest.json"


def _esc(s: str) -> str:
    """Escape an untrusted spider-derived string for an adj-lang `source "..."` literal.
    Backslash FIRST (a trailing `\\` would escape the closing quote and corrupt the parse of
    every following clause), then the double-quote, then collapse control chars to spaces."""
    s = s.replace("\\", "\\\\").replace('"', '\\"')
    return re.sub(r"[\r\n\t]", " ", s)


# ---------------------------------------------------------------------------
# The dose-cap knowledge, as data.
#
#   risk categories:  a graded chart token (hepatic_severe) belongs to a category (hepatic).
#                     Definitional — which severities count as a given organ-impairment — so
#                     no grounding (rendered bare, like `has_class`).
#   compound risks:   a compound factor holds when TWO categories are both present.  The pair
#                     is the structural definition of the compound (e.g. hepatorenal).
#   dose caps:        a drug is dose-capped under a (usually compound) risk.  GROUNDED — this
#                     is the clinical claim, carrying the label byte-quote.
# ---------------------------------------------------------------------------

# (risk token, category) — definitional membership of a graded token in an organ category.
RISK_CATEGORIES: list[tuple[str, str]] = [
    ("hepatic_severe", "hepatic"),
    ("hepatic_moderate", "hepatic"),
    ("renal_severe", "renal"),
    ("renal_moderate", "renal"),
]

# (compound token, category_a, category_b) — a two-factor compound risk.  It holds for a
# patient iff some active risk is in category_a AND some active risk is in category_b.
COMPOUND_RISKS: list[tuple[str, str, str]] = [
    ("hepatorenal", "hepatic", "renal"),
]

# (grounding_id, drug, compound/risk token, authored_fallback_quote) — the GROUNDED cap.
DOSE_CAPS: list[tuple[str, str, str, str]] = [
    ("dose_cap_ceftriaxone_hepatorenal", "ceftriaxone", "hepatorenal",
     "Ceftriaxone is capped (no more than 2 g/day) in patients who have BOTH hepatic "
     "impairment and significant renal impairment (FDA label)."),
]


def _gate(rec: dict | None) -> tuple[str, str]:
    """Map a grounding record to (verdict, trust).  A dose cap is directional, so a `grounded`
    OR `direction_only` verdict WITH a byte-quote grounds it authoritatively; any other state
    is authored-debt at consensus trust."""
    if not rec:
        return "FLAG:pending", "consensus"
    g = rec.get("grounded") or {}
    verdict = g.get("verdict")
    if verdict in ("grounded", "direction_only") and g.get("byte_quote"):
        return "ACCEPT", "authoritative"
    return f"FLAG:{re.sub(r'[^a-z_]', '', (verdict or 'pending').lower())[:24] or 'pending'}", "consensus"


def _cap_block(drug: str, risk: str, gid: str, authored: str,
               rec: dict | None) -> tuple[str, dict]:
    """Render one grounded `relate dose_capped_under(<drug>, <risk>)` clause + manifest entry."""
    verdict, trust = _gate(rec)
    g = (rec or {}).get("grounded") or {}
    if verdict == "ACCEPT":
        source = g.get("byte_quote") or authored
        url = g.get("resolved_url")
        lines = [f"    relate dose_capped_under({drug}, {risk})",
                 f'        source "{_esc(source)}"']
        if url:
            lines.append(f'        locator "{_esc(url)}"')
        lines.append("        trust authoritative")
    else:
        source, url = authored, None
        lines = [f"    relate dose_capped_under({drug}, {risk})",
                 f'        source "{_esc(authored)}"',
                 f"        trust consensus   % [{verdict}]"]
    entry = {"relation": "dose_capped_under", "subject": drug, "risk": risk,
             "grounding_id": gid, "verdict": verdict, "trust": trust,
             "source": source, "url": url}
    return "\n".join(lines), entry


HEADER = """\
% ============================================================================
% dose_caps — CONJUNCTIVE dose ceilings as risk-scoped derivation rules (ADJ-native).
% ============================================================================
% MYCIN-2026 (therapy layer, CC-2b).  Some dose ceilings fire only on a CONJUNCTION
% of patient risk factors.  The ceftriaxone FDA label: "Patients with hepatic
% impairment AND significant renal impairment should not receive more than 2 grams
% per day of ceftriaxone."  Hepatic impairment ALONE needs no adjustment — so the
% trigger is the joint condition, not either factor singly.
%
% A compound risk holds when two categories are both active; a drug is capped under
% a risk.  The engine joins the patient's active risks with these rules and DERIVES
% the compound risk + the capped drug:
%
%     ? derived_risk($R)        % which compound risks hold for this patient
%     ? dose_capped($D, $R)     % which drugs are dose-capped (carries the byte-quote)
%
% This replaces a hand-written Python conjunction (`if has_hepatic and has_renal`) —
% the reasoning now lives in the language, not the compiler.  The SAME generic shape
% encodes ANY two-factor compound risk and any drug capped under it (add a row, not a
% branch): a domain-neutral substrate (project_adj_universal_rule_substrate).
%
% GENERATED by treatment/antibiotics/dose_caps_build.py from
% grounding/dose-window-grounding.json (the spider's byte-provenanced output).  Do not
% hand-edit — re-ground and regenerate.  A grounded cap carries its byte-quote as
% `source` + `trust authoritative` (+ the fetched URL as `locator`); an ungrounded one
% stays `trust consensus` tagged `% [FLAG: …]` so the authored-debt is visible and drives
% to zero (feedback_nothing_human_authored).  NOTE: the cap's DIRECTION (a ceiling on this
% drug under this conjunction) is grounded; the precise mg/kg shrink lives in formulary.json
% and is an ILLUSTRATIVE feasibility model, not validated PK/PD.
% ============================================================================

dictionary dose_cap_vocab {
    define drug          : entity   surface "drug", "antibiotic", "agent"
    define risk_factor   : entity   surface "risk factor", "patient risk"
    define risk_category : entity   surface "risk category", "organ category"

    % a graded risk token belongs to an organ category (definitional membership).
    define risk_in_category  : relation from risk_factor to risk_category
    % a compound risk's two component categories (structural definition).
    define compound_first    : relation from risk_factor to risk_category
    define compound_second   : relation from risk_factor to risk_category
    % a drug's dose is capped under a (usually compound) risk (the grounded claim).
    define dose_capped_under : relation from drug to risk_factor
    % the patient's currently-active risk tokens (asserted per case from the chart).
    define active_risk       : relation from risk_factor to risk_factor
    % DERIVED: a compound risk that holds for this patient.
    define derived_risk      : relation from risk_factor to risk_factor
    % DERIVED: drug D's dose is capped under risk R (the query target; carries the quote).
    define dose_capped       : relation from drug to risk_factor
}
"""

# The two generic derivation rules.  STRUCTURAL: "a compound risk holds when both its
# categories have an active member" and "a drug is capped when a compound risk it is capped
# under holds".  Both go straight from base facts (no rule-on-rule chaining) so they need only
# single-pass SLD.  The clinical grounding lives on the `dose_capped_under` fact they join,
# whose byte-quote flows into each `dose_capped` answer's citations.
RULES = """\

    % --- the GENERIC conjunction rules ---------------------------------------
    % a compound risk holds when an active risk is in its first category AND a
    % (necessarily different-category) active risk is in its second category.
    rule {
        head: derived_risk($C)
        when: compound_first($C, $CatA), risk_in_category($Ra, $CatA), active_risk($Ra),
              compound_second($C, $CatB), risk_in_category($Rb, $CatB), active_risk($Rb)
    }
    % a drug is dose-capped when it is capped under a compound risk that holds.
    rule {
        head: dose_capped($D, $C)
        when: compound_first($C, $CatA), risk_in_category($Ra, $CatA), active_risk($Ra),
              compound_second($C, $CatB), risk_in_category($Rb, $CatB), active_risk($Rb),
              dose_capped_under($D, $C)
    }
"""


def build(check: bool = False) -> int:
    recs: dict[str, dict] = {}
    if GROUNDING.exists():
        recs = {r["id"]: r for r in json.loads(GROUNDING.read_text()).get("records", [])}

    body = [HEADER, "rulebook dose_caps {", "    use dose_cap_vocab"]
    clauses: dict[str, dict] = {}

    body.append("")
    body.append("    % --- risk token → organ category (definitional) " + "-" * 24)
    for tok, cat in RISK_CATEGORIES:
        body.append(f"    relate risk_in_category({tok}, {cat})")
        clauses[f"risk_in_category__{tok}__{cat}"] = {
            "relation": "risk_in_category", "subject": tok, "risk": cat,
            "verdict": "DEFINITIONAL", "trust": "definitional", "grounding_id": None}

    body.append("")
    body.append("    % --- compound risks = two component categories (structural) " + "-" * 13)
    for comp, a, b in COMPOUND_RISKS:
        body.append(f"    relate compound_first({comp}, {a})")
        body.append(f"    relate compound_second({comp}, {b})")
        clauses[f"compound__{comp}"] = {
            "relation": "compound", "subject": comp, "risk": f"{a}+{b}",
            "verdict": "DEFINITIONAL", "trust": "definitional", "grounding_id": None}

    body.append("")
    body.append("    % --- grounded dose caps (the clinical claim) " + "-" * 28)
    for gid, drug, risk, authored in DOSE_CAPS:
        block, entry = _cap_block(drug, risk, gid, authored, recs.get(gid))
        body.append(block)
        clauses[f"dose_capped_under__{drug}__{risk}"] = entry

    body.append(RULES.rstrip("\n"))
    body.append("}")
    adj_text = "\n".join(body) + "\n"

    accepted = sum(1 for c in clauses.values() if c["verdict"] == "ACCEPT")
    flagged = sum(1 for c in clauses.values() if str(c["verdict"]).startswith("FLAG"))
    manifest = {"kind": "dose_cap", "clauses": clauses,
                "hash": hashlib.sha256(json.dumps(clauses, sort_keys=True).encode()).hexdigest()[:16]}

    if check:
        ok = ADJ.exists() and ADJ.read_text() == adj_text
        mok = MANIFEST.exists() and json.loads(MANIFEST.read_text()).get("hash") == manifest["hash"]
        print("dose_caps_build --check:", "up to date" if (ok and mok) else "OUT OF DATE")
        return 0 if (ok and mok) else 1

    ADJ.write_text(adj_text)
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"dose_caps_build: regenerated dose_caps.adj + manifest "
          f"({accepted} grounded ACCEPT, {flagged} consensus/flagged authored-debt).")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
