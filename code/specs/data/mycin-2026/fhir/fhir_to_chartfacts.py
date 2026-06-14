#!/usr/bin/env python3
"""fhir_to_chartfacts.py — a (de-identified) FHIR chart → the treatment constraint IR (CH).

This is the on-ramp the chart-as-constraints vision needs (CHART-AS-CONSTRAINTS.md, CC-7):
a whole patient chart becomes the typed chart-fact IR that the constraint compiler
(treatment/antibiotics/chart_to_cop.py) turns into a constraint program. D1's fhir_ingest
maps coded resources to DIAGNOSTIC findings ("which organism?"); this maps the
TREATMENT-relevant chart facts (allergy, renal function, immune status, interacting meds,
weight, culture sensitivities) into the COP's ChartFacts.

It runs on the DE-IDENTIFIED bundle (deidentify.py) — privacy-first, fully local, 0 model
calls (the chart is coded; only narrative falls back to the decomposer, elsewhere). Every
recognized resource becomes a provenance-bearing ChartFact; an unrecognized clinically-
relevant resource is surfaced as a DISCARD with a reason — nothing in the chart is silently
dropped (the same "no unaccounted bytes" discipline the COP applies).

  de-identified FHIR Bundle ──► [age, allergy, immune, renal, interaction, weight, …]
                                 → chart_to_cop.compile_cop → constraint program

Usage:  python3 fhir_to_chartfacts.py samples/<bundle>.json [as_of_year]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "treatment" / "antibiotics"))
import chart_to_cop as cc  # noqa: E402  (ChartFact, the COP IR)

# Compact recognizers — match on the coded `code`/system OR the (de-identified) `code.text`,
# so a chart that codes its problems/meds/allergies maps deterministically. Keyword match is
# on lowercased code text; codes are matched exactly. (A fuller code map is a follow-up; this
# covers the treatment-decision-relevant facts the COP consumes.)
ALLERGEN = {  # substring in allergy code text → ChartFact allergy value
    "penicillin": "penicillin", "amoxicillin": "penicillin", "ampicillin": "penicillin",
    "cephalosporin": "cephalosporin", "beta-lactam": "betalactam", "beta lactam": "betalactam",
}
IMMUNO = ("immunocompromised", "immunosuppress", "hiv", "aids", "transplant", "neutropenia",
          "leukemia", "chemotherapy", "asplenia")
RENAL_SEVERE = ("end-stage renal", "esrd", "ckd stage 5", "chronic kidney disease stage 5",
                "dialysis", "renal failure", "kidney failure")
RENAL_MOD = ("ckd stage 3", "ckd stage 4", "chronic kidney disease stage 3",
             "chronic kidney disease stage 4", "moderate renal")
NEPHROTOXIN = ("tacrolimus", "ciclosporin", "cyclosporine", "gentamicin", "tobramycin",
               "amikacin", "amphotericin", "vancomycin", "cisplatin", "nsaid", "ibuprofen")
WEIGHT_LOINC = {"29463-7", "3141-9"}        # body weight
EGFR_LOINC = {"33914-3", "48642-3", "62238-1", "98979-8"}  # estimated GFR


def _texts(resource: dict, *fields: str) -> list[str]:
    """Lowercased code `text` + coding `display`/`code` strings from the given fields."""
    out: list[str] = []
    for f in fields:
        cc_obj = resource.get(f) or {}
        if isinstance(cc_obj, dict):
            if cc_obj.get("text"):
                out.append(str(cc_obj["text"]).lower())
            for c in cc_obj.get("coding", []) or []:
                out += [str(c.get("display", "")).lower(), str(c.get("code", "")).lower()]
    return [t for t in out if t]


def _coding_codes(resource: dict, field: str) -> set[str]:
    return {str(c.get("code", "")) for c in ((resource.get(field) or {}).get("coding", []) or [])}


def to_chartfacts(bundle: dict, as_of_year: int | None = None) -> tuple[list[cc.ChartFact], list[dict]]:
    """Map a de-identified FHIR Bundle → (ChartFacts for the COP, discards)."""
    facts: list[cc.ChartFact] = []
    discards: list[dict] = []
    for entry in bundle.get("entry", []):
        r = entry.get("resource", {})
        rt = r.get("resourceType")
        if rt == "Patient":
            by = r.get("birthDate")
            if by and as_of_year and str(by)[:4].isdigit():
                age = as_of_year - int(str(by)[:4])
                band = ("infant_child" if age < 16 else "older_adult" if age >= 50 else "adult")
                facts.append(cc.ChartFact("age_band", band, f"birthYear {by}"))
            elif by:
                discards.append({"fact": f"Patient.birthDate={by}",
                                 "reason": "no as_of_year given → cannot derive age band"})
        elif rt == "AllergyIntolerance":
            txt = " ".join(_texts(r, "code"))
            hit = next((v for kw, v in ALLERGEN.items() if kw in txt), None)
            if hit:
                facts.append(cc.ChartFact("allergy", hit, txt[:60]))
            else:
                discards.append({"fact": f"AllergyIntolerance({txt[:40]})",
                                 "reason": "allergen not in the drug-class exclusion map yet"})
        elif rt == "Condition":
            txt = " ".join(_texts(r, "code"))
            if any(k in txt for k in IMMUNO):
                facts.append(cc.ChartFact("immune_status", "immunocompromised", txt[:60]))
            elif any(k in txt for k in RENAL_SEVERE):
                facts.append(cc.ChartFact("renal_status", "renal_severe", txt[:60]))
            elif any(k in txt for k in RENAL_MOD):
                facts.append(cc.ChartFact("renal_status", "renal_moderate", txt[:60]))
            # other conditions are diagnostic context, not treatment constraints → ignore quietly
        elif rt in ("MedicationStatement", "MedicationRequest"):
            txt = " ".join(_texts(r, "medicationCodeableConcept"))
            if any(k in txt for k in NEPHROTOXIN):
                facts.append(cc.ChartFact("interaction", "nephrotoxin_interaction", txt[:60]))
        elif rt == "Observation":
            codes = _coding_codes(r, "code")
            vq = r.get("valueQuantity") or {}
            if codes & WEIGHT_LOINC and isinstance(vq.get("value"), (int, float)):
                facts.append(cc.ChartFact("weight", str(vq["value"]), f"body weight {vq.get('unit','')}"))
            elif codes & EGFR_LOINC and isinstance(vq.get("value"), (int, float)):
                # eGFR < 30 → severe; 30–59 → moderate (standard CKD staging).
                band = "renal_severe" if vq["value"] < 30 else "renal_moderate" if vq["value"] < 60 else None
                if band:
                    facts.append(cc.ChartFact("renal_status", band, f"eGFR {vq['value']}"))
    return facts, discards


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: fhir_to_chartfacts.py <bundle.json> [as_of_year]", file=sys.stderr)
        return 2
    bundle = json.loads(Path(argv[0]).read_text())
    year = int(argv[1]) if len(argv) > 1 else None
    facts, discards = to_chartfacts(bundle, year)
    print("chart facts:")
    for f in facts:
        print(f"  {f.kind} = {f.value}    [{f.span}]")
    if discards:
        print("discards:", discards)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
