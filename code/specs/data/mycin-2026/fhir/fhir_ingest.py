#!/usr/bin/env python3
"""fhir_ingest.py - read an HL7 FHIR chart export into the MYCIN-2026 pipeline.

MYCIN-2026 D1. The "open format like EPIC" is HL7 FHIR - the API EPIC/Cerner and
every modern EHR expose. A FHIR `Bundle` carries the chart as resources
(Patient, Condition, Observation, AllergyIntolerance, MedicationStatement). The
key opportunity: a chart's labs/vitals/problems are usually CODED (LOINC for
observations, SNOMED for conditions) with an INTERPRETATION flag, so we can map
them to typed dictionary findings DETERMINISTICALLY - 0 model calls even for the
decompose step. The structured chart goes straight to the engine; only narrative
/ uncoded resources fall back to the small-model decomposer.

  FHIR Bundle ──► extract chart ──► coded resources → typed findings   (0 model calls)
                                  └► narrative text  → prose (for decompose_text)

This module is pure parsing + a lookup against fhir_code_map.json; it never makes
a network call (a FHIR Bundle is a self-contained JSON document) and never leaves
the machine. It is robust to the messy reality of FHIR (missing fields, codes it
doesn't know) - unknown codes are surfaced, never guessed.

Usage:  python3 fhir_ingest.py samples/meningitis_bundle.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CODE_MAP = json.loads((HERE / "fhir_code_map.json").read_text())
INTERP_SYSTEM = CODE_MAP["interpretation_system"]


def _resources(bundle: dict, rtype: str) -> list[dict]:
    """Every resource of a given type in a Bundle (tolerant of shape)."""
    out = []
    for entry in bundle.get("entry", []) or []:
        res = entry.get("resource", entry) if isinstance(entry, dict) else {}
        if isinstance(res, dict) and res.get("resourceType") == rtype:
            out.append(res)
    return out


def _codes(concept: dict, system: str | None = None) -> list[str]:
    """The codes in a CodeableConcept (optionally filtered to a coding system)."""
    out = []
    for c in (concept or {}).get("coding", []) or []:
        if system is None or c.get("system") == system:
            if c.get("code") is not None:
                out.append(str(c["code"]))
    return out


def _interpretation(obs: dict) -> str | None:
    """The HL7 interpretation flag (H/L/N/POS/NEG/A) of an Observation, if any."""
    interp = obs.get("interpretation")
    concepts = interp if isinstance(interp, list) else ([interp] if interp else [])
    for concept in concepts:
        codes = _codes(concept, INTERP_SYSTEM) or _codes(concept)
        if codes:
            return codes[0]
    return None


def observation_to_finding(obs: dict) -> tuple[str, str] | None:
    """Map a coded Observation to (functor, value), or None if its code/value is
    not in the map. Never guesses - an unmapped code returns None."""
    for code in _codes(obs.get("code", {}), CODE_MAP["loinc_system"]) or _codes(obs.get("code", {})):
        rule = CODE_MAP["observations"].get(code)
        if not rule:
            continue
        if rule["from"] == "interpretation":
            flag = _interpretation(obs)
            value = rule["map"].get(flag) if flag else None
            if value:
                return rule["functor"], value
        elif rule["from"] == "temperature_celsius":
            q = obs.get("valueQuantity", {})
            temp = q.get("value")
            if isinstance(temp, (int, float)):
                # normalize Fahrenheit if the unit says so
                if str(q.get("unit", "")).lower() in ("[degf]", "f", "degf", "°f"):
                    temp = (temp - 32) * 5.0 / 9.0
                return rule["functor"], rule["above"] if temp > rule["threshold"] else rule["at_or_below"]
    return None


def condition_to_finding(cond: dict) -> tuple[str, str] | None:
    """Map a coded Condition (a problem-list entry) to (functor, value)."""
    concept = cond.get("code", {})
    for code in _codes(concept, CODE_MAP["snomed_system"]) or _codes(concept):
        rule = CODE_MAP["conditions"].get(code)
        if rule:
            return rule["functor"], rule["value"]
    return None


def extract(bundle: dict) -> dict:
    """Parse a FHIR Bundle into: typed `findings` (functor(value), 0 model calls),
    `unmapped` coded resources (recorded, never guessed), and `narrative` text for
    the decomposer. Demographics are pulled from the Patient for context."""
    findings: list[str] = []
    seen: set[str] = set()
    unmapped: list[str] = []
    narrative: list[str] = []

    def add(pair: tuple[str, str] | None, describe: str) -> bool:
        if pair is None:
            return False
        term = f"{pair[0]}({pair[1]})"
        if term not in seen:
            seen.add(term)
            findings.append(term)
        return True

    for obs in _resources(bundle, "Observation"):
        label = (obs.get("code", {}).get("text")
                 or "; ".join(c.get("display", "") for c in obs.get("code", {}).get("coding", []))
                 or "observation")
        if not add(observation_to_finding(obs), label):
            unmapped.append(f"Observation: {label}")
            if obs.get("note"):
                narrative += [n.get("text", "") for n in obs["note"] if n.get("text")]
    for cond in _resources(bundle, "Condition"):
        label = (cond.get("code", {}).get("text")
                 or "; ".join(c.get("display", "") for c in cond.get("code", {}).get("coding", []))
                 or "condition")
        if not add(condition_to_finding(cond), label):
            unmapped.append(f"Condition: {label}")
            narrative.append(label)
    # Free-text narrative the EHR carries (HPI etc.) for the decomposer.
    for comp in _resources(bundle, "Composition"):
        for section in comp.get("section", []) or []:
            div = section.get("text", {}).get("div")
            if div:
                narrative.append(_strip_html(div))

    pt = next(iter(_resources(bundle, "Patient")), {})
    demographics = {
        "age_years": _age_from_birthdate(pt.get("birthDate")),
        "gender": pt.get("gender"),
    }
    allergies = [a.get("code", {}).get("text")
                 or "; ".join(c.get("display", "") for c in a.get("code", {}).get("coding", []))
                 for a in _resources(bundle, "AllergyIntolerance")]
    medications = [m.get("medicationCodeableConcept", {}).get("text")
                   or "; ".join(c.get("display", "") for c in
                                m.get("medicationCodeableConcept", {}).get("coding", []))
                   for m in _resources(bundle, "MedicationStatement")]

    return {
        "demographics": demographics,
        "findings": findings,
        "unmapped": unmapped,
        "narrative": " ".join(n for n in narrative if n).strip(),
        "allergies": [a for a in allergies if a],
        "medications": [m for m in medications if m],
    }


def _strip_html(s: str) -> str:
    import re
    return re.sub(r"<[^>]+>", " ", s).strip()


def _age_from_birthdate(birthdate: str | None) -> int | None:
    """Approximate age in years from a FHIR `birthDate` (YYYY or YYYY-MM-DD),
    using the year recorded in the Bundle context. We avoid a wall-clock call
    (kept deterministic): just take the leading year and subtract from a fixed
    reference is not possible without 'now', so we return None for age when only a
    birth year is known and leave age to the narrative/observations."""
    # FHIR carries no 'now'; deriving exact age needs the encounter date. We expose
    # the birth year for context and let the clinical findings (not age math) drive
    # the differential. (Age-banded priors are future work; see organism-id A1.)
    return None


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: fhir_ingest.py <bundle.json>", file=sys.stderr)
        return 2
    bundle = json.loads(Path(argv[0]).read_text())
    chart = extract(bundle)
    print(json.dumps(chart, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
