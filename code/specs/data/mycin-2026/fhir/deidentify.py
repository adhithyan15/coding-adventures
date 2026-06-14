#!/usr/bin/env python3
"""deidentify.py — strip PHI from a FHIR chart before it enters the pipeline (CH).

The chart-as-constraints vision needs a WHOLE patient chart, but MYCIN-2026 is local
and privacy-first: the clinical SIGNAL (codes, values, interpretation flags) is what the
engine reasons over — the patient's IDENTITY is not. So we de-identify the FHIR Bundle
up front, deterministically, following the HIPAA **Safe Harbor** method: remove the 18
classes of identifiers, generalize dates to the year, and aggregate ages over 89.

This is **de-identification only** — it strips identity, it never re-identifies. There is
no linkage table, no re-identification key; the mapping is one-way by construction. It runs
locally and makes no network call (a Bundle is self-contained JSON).

  raw FHIR Bundle ──► deidentify ──► de-identified Bundle (+ a report of what was removed)
                                     → fhir_to_chartfacts → the constraint optimizer

What is REMOVED wherever it appears (names, contacts, identifiers, free text):
  name · telecom · address · identifier · photo · contact · the Narrative `text` div ·
  `note` (Annotation free text — the richest PHI field) · meta `source` · Patient `link` ·
  deceased flags · multipleBirth · a Reference's `display` (a party name) · the extension
  PHI value-types (valueHumanName/valueAddress/valueContactPoint/valueIdentifier) ·
  Provenance authorship (authorString/authorReference).
What is GENERALIZED: EVERY date/dateTime-shaped value → its year — by value shape, not key
  name, so Period/`value[x]`/timing/extension dates can't slip a key allow-list (Safe Harbor
  §(C)); with `as_of_year`, a birth year implying age > 89 is aggregated to a 90+ sentinel.
What is KEPT: gender, the clinical `code`s (LOINC/SNOMED/RxNorm), `value*`, `interpretation`,
  `clinicalStatus`, reaction codes, and the clinical labels `CodeableConcept.text` /
  `Coding.display` — the signal the engine needs. KNOWN LIMITATION: those kept labels are NOT
  scrubbed for PHI embedded in a locally-authored free-text label (NER redaction is out of
  scope); `report['free_text_kept']` counts them so the residual surface is auditable.

Usage:  python3 deidentify.py samples/<bundle>.json
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

# The Safe Harbor drop set — keys removed wherever they occur in any resource. Includes the
# extension PHI value-types (valueHumanName/Address/ContactPoint/Identifier), the free-text
# `note` (Annotation — the single richest PHI-bearing field: names, dates, "lives with…"),
# meta.source (a source-system URI), Provenance authorship, deceased flags, and Patient.link
# (a relationship/linkage identifier). NOTE: `text` and `display` are handled specially
# (below), NOT dropped wholesale: a resource-level `text` is the PHI-bearing Narrative
# ({status, div}), but a CodeableConcept's `text` ("Penicillin") is a clinical label kept for
# the engine; a Coding's `display` is clinical, but a Reference's `display` can echo a name.
PHI_KEYS = frozenset({
    "name", "telecom", "address", "identifier", "photo", "contact",
    "deceasedDateTime", "deceasedBoolean", "multipleBirthInteger", "multipleBirthBoolean",
    "patient_name", "mrn", "ssn", "email", "phone", "link", "note", "source",
    "valueHumanName", "valueAddress", "valueContactPoint", "valueIdentifier",
    "valueString", "valueMarkdown", "authorString", "authorReference",
})
# (Free-text `valueString`/`valueMarkdown` are dropped wherever they occur — in an extension
# they carry metadata PHI, and as an Observation.value[x] they are unscrubbed free text. The
# engine reasons over coded values / `valueQuantity` / `valueCodeableConcept`, so dropping the
# free-text value is the privacy-safe choice; an Observation left with no coded value is
# surfaced as uncoded downstream, never guessed.)
# Keys that are ALWAYS a date → generalized to the year even if value-shape detection is
# unsure. Date generalization is primarily by VALUE SHAPE (any ISO-date-looking string,
# wherever it sits — period.start, valueDateTime, an extension date), so Period/value[x]/
# timing dates can't slip through a key allow-list (Safe Harbor §(C)).
DATE_KEYS = frozenset({
    "birthDate", "effectiveDateTime", "onsetDateTime", "recordedDate", "recorded", "issued",
    "authoredOn", "date", "started", "performedDateTime", "occurrenceDateTime",
    "abatementDateTime", "assertedDate", "lastUpdated", "start", "end", "time",
})
_YEAR_RE = re.compile(r"\b(\d{4})\b")
# An ISO-8601 date / dateTime leaf — matched anywhere to generalize to the year.
_ISO_DATE_RE = re.compile(r"^\d{4}-\d{2}(-\d{2})?([T ]\d{2}:\d{2}\S*)?$")
AGE_CAP = 89   # Safe Harbor §(C): ages over 89 (and date elements implying them) are aggregated.


def _year(value: object) -> str | None:
    """The year of a FHIR date/dateTime string, or None if no 4-digit year is present."""
    m = _YEAR_RE.search(str(value)) if value is not None else None
    return m.group(1) if m else None


def _scrub(node: object, report: dict, as_of_year: int | None) -> object:
    """Recursively de-identify a FHIR node: drop PHI keys, generalize EVERY date-shaped
    value to its year (by shape, not key name), and cap birth years implying age > 89."""
    if isinstance(node, dict):
        is_reference = "reference" in node          # a Reference object (vs a Coding)
        out = {}
        for k, v in node.items():
            if k in PHI_KEYS:
                report["removed"][k] = report["removed"].get(k, 0) + 1
                continue
            if k == "text":
                # Narrative (a {status, div} dict) is free text that can carry names/dates →
                # drop it; a CodeableConcept.text (a string) is the clinical label → keep.
                if isinstance(v, dict):
                    report["removed"]["narrative"] = report["removed"].get("narrative", 0) + 1
                    continue
                out[k] = v
                continue
            if k == "display" and is_reference:
                # Reference.display may echo a person/org name → drop (Coding.display kept).
                report["removed"]["display"] = report["removed"].get("display", 0) + 1
                continue
            if k == "birthDate":
                y = _year(v)
                if y is None:
                    continue
                # Safe Harbor age cap: a birth year implying age > 89 is aggregated to 90+
                # (sentinel year) so no >89 age can be derived from the de-identified artifact.
                if as_of_year is not None and as_of_year - int(y) > AGE_CAP:
                    out["birthDate"] = "1900"
                    report["age_capped"] += 1
                else:
                    out["birthDate"] = y          # year-only partial date (age still derivable)
                report["dates_generalized"] += 1
                continue
            out[k] = _scrub(v, report, as_of_year)
        return out
    if isinstance(node, list):
        return [_scrub(x, report, as_of_year) for x in node]
    # A date/dateTime LEAF anywhere (period.start, valueDateTime, an extension date) →
    # generalize to the year so no exact date survives a key the allow-list didn't name.
    if isinstance(node, str) and _ISO_DATE_RE.match(node):
        report["dates_generalized"] += 1
        return _year(node) or ""
    return node


def deidentify(bundle: dict, as_of_year: int | None = None) -> tuple[dict, dict]:
    """De-identify a FHIR Bundle (HIPAA Safe Harbor). Returns (de-identified bundle, report).
    One-way: there is no re-identification key. The clinical codes/values/interpretation are
    preserved; identity (names, contacts, identifiers, narrative, exact dates) is removed.

    `as_of_year` enables the Safe Harbor age cap: a birth year implying age > 89 is collapsed
    to a 90+ sentinel so the de-identified artifact itself can't reveal a >89 age. Without it,
    the birth year is kept (still year-only); pass it whenever the chart's reference year is
    known.

    KNOWN LIMITATION (documented, not silent): free-text CLINICAL fields kept for the engine —
    `CodeableConcept.text` and `Coding.display` — are NOT scrubbed for embedded PHI. A locally
    authored label like "Maria's UTI" would survive. Safe Harbor de-id of free text needs NER
    redaction, out of scope here; the high-yield free-text fields (Narrative, `note`/Annotation)
    ARE dropped. `report['free_text_kept']` counts the preserved labels so the surface is auditable."""
    report = {"removed": {}, "dates_generalized": 0, "age_capped": 0, "free_text_kept": 0}
    clean = _scrub(bundle, report, as_of_year)
    # Audit the preserved-free-text surface (CodeableConcept.text / Coding.display).
    def _count(n):
        if isinstance(n, dict):
            for k, v in n.items():
                if k in ("text", "display") and isinstance(v, str):
                    report["free_text_kept"] += 1
                _count(v)
        elif isinstance(n, list):
            for x in n:
                _count(x)
    _count(clean)
    return clean, report


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: deidentify.py <bundle.json> [as_of_year]", file=sys.stderr)
        return 2
    bundle = json.loads(Path(argv[0]).read_text())
    year = int(argv[1]) if len(argv) > 1 else None
    clean, report = deidentify(bundle, year)
    removed = ", ".join(f"{k}×{n}" for k, n in sorted(report["removed"].items()))
    print(f"deidentify: removed [{removed or 'nothing'}]; "
          f"generalized {report['dates_generalized']} date(s) to year.")
    print(json.dumps(clean, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
