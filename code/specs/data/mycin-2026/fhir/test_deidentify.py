#!/usr/bin/env python3
"""test_deidentify.py — guard PHI de-identification + the chart→constraint on-ramp (CH).

The privacy-critical test: NO PHI token from the source chart may survive de-identification,
the PHI keys must be gone, dates must be generalized to the year — while the CLINICAL signal
(codes, values, interpretation, code labels) is preserved. Then the deterministic chart→
ChartFact bridge + the constraint optimizer: a de-identified complex chart (ESRD + a
nephrotoxin + a penicillin allergy) drives the optimizer to an honest ABSTENTION, 0 model
calls — the full charting → constraints path, privacy-first.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "treatment" / "antibiotics"))
sys.path.insert(0, str(HERE.parent / "warm"))
import deidentify  # noqa: E402
import fhir_to_chartfacts as f2c  # noqa: E402
import chart_to_cop as cc  # noqa: E402
import decide as decide_mod  # noqa: E402

SAMPLE = HERE / "samples" / "chart_with_phi_bundle.json"
# Exact PHI tokens planted in the sample — none may appear anywhere in the de-identified output.
PHI_TOKENS = ["Hernandez", "Maria", "Elena", "MRN-4481920", "555-21-7788", "415-555-0173",
              "maria.hernandez@example.com", "Pine Street", "Apt 6B", "94108",
              "2026-06-14T08:30:00Z", "2019-03-02", "1968-04-12"]


def _walk_keys(node):
    if isinstance(node, dict):
        for k, v in node.items():
            yield k
            yield from _walk_keys(v)
    elif isinstance(node, list):
        for x in node:
            yield from _walk_keys(x)


def test_no_phi_survives():
    raw = json.loads(SAMPLE.read_text())
    clean, report = deidentify.deidentify(raw)
    blob = json.dumps(clean)
    for tok in PHI_TOKENS:
        assert tok not in blob, f"PHI LEAK: {tok!r} survived de-identification"
    # The Safe Harbor identifier keys are gone from every resource.
    keys = set(_walk_keys(clean))
    for phi_key in ("name", "telecom", "address", "identifier"):
        assert phi_key not in keys, f"PHI key {phi_key!r} not removed"
    # Narrative removed; birthDate reduced to a bare year.
    pat = next(e["resource"] for e in clean["entry"] if e["resource"]["resourceType"] == "Patient")
    assert "text" not in pat and pat.get("birthDate") == "1968"
    assert report["dates_generalized"] >= 1


def test_clinical_signal_preserved():
    clean, _ = deidentify.deidentify(json.loads(SAMPLE.read_text()))
    blob = json.dumps(clean)
    # Clinical labels (CodeableConcept.text / Coding.display) and coded values are KEPT —
    # de-identification strips identity, not the medicine.
    for kept in ("Penicillin", "End-stage renal disease", "33914-3", "Estimated GFR"):
        assert kept in blob, f"clinical signal {kept!r} was wrongly removed"
    # The eGFR value + interpretation the engine reasons over survive.
    assert '"value": 9' in blob


def test_deidentify_does_not_mutate_input_and_is_idempotent():
    raw = json.loads(SAMPLE.read_text())
    before = json.dumps(raw)
    clean1, _ = deidentify.deidentify(raw)
    assert json.dumps(raw) == before, "deidentify must not mutate its input"
    clean2, _ = deidentify.deidentify(clean1)
    assert json.dumps(clean1) == json.dumps(clean2), "deidentify must be idempotent"


def test_residual_phi_paths_closed():
    # The hard cases a key-allow-list misses: Period/value[x]/extension dates, extension
    # free text, and Annotation `note` — all must be scrubbed (privacy review hardening).
    res = {"entry": [{"resource": {
        "resourceType": "Encounter",
        "period": {"start": "2024-07-15T13:22:00Z", "end": "2024-07-18"},
        "extension": [{"url": "http://x/employer", "valueString": "Acme Corp, contact Maria 415-555-0173"},
                      {"url": "http://x/onset", "valueDateTime": "2024-07-15"}],
        "note": [{"text": "Patient Maria Hernandez lives at 482 Pine St", "time": "2024-07-15T13:22:00Z"}],
    }}]}
    clean, rep = deidentify.deidentify(res, as_of_year=2026)
    blob = json.dumps(clean)
    for leak in ("2024-07-15T13:22", "2024-07-18", "Acme Corp", "Maria", "415-555", "Pine St", "13:22"):
        assert leak not in blob, f"residual PHI leak: {leak!r}"
    assert '"start": "2024"' in blob, "period dates must be generalized to year, not dropped"


def test_age_over_89_is_capped():
    raw = {"entry": [{"resource": {"resourceType": "Patient", "birthDate": "1930-05-01"}}]}
    clean, rep = deidentify.deidentify(raw, as_of_year=2026)   # 96yo → 90+
    bd = clean["entry"][0]["resource"]["birthDate"]
    assert bd == "1900" and rep["age_capped"] == 1, (bd, rep)
    # Under-89 keeps the year (still derivable, ±1).
    clean2, rep2 = deidentify.deidentify(
        {"entry": [{"resource": {"resourceType": "Patient", "birthDate": "1970-01-01"}}]}, as_of_year=2026)
    assert clean2["entry"][0]["resource"]["birthDate"] == "1970" and rep2["age_capped"] == 0


def test_chart_to_chartfacts_mapping():
    clean, _ = deidentify.deidentify(json.loads(SAMPLE.read_text()))
    facts, discards = f2c.to_chartfacts(clean, as_of_year=2026)
    kinds = {(x.kind, x.value) for x in facts}
    assert ("allergy", "penicillin") in kinds
    assert ("interaction", "nephrotoxin_interaction") in kinds
    assert ("renal_status", "renal_severe") in kinds   # from ESRD condition + eGFR 9
    assert ("age_band", "older_adult") in kinds        # birthYear 1968, as_of 2026
    assert any(x.kind == "weight" for x in facts)


def test_end_to_end_complex_chart_abstains():
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_deidentify: PASS (de-id + mapping); engine portion SKIPPED (no cli)")
        return
    clean, _ = deidentify.deidentify(json.loads(SAMPLE.read_text()))
    facts, _ = f2c.to_chartfacts(clean, as_of_year=2026)
    r = cc.derive(cli, facts)
    # β-lactams excluded by the penicillin allergy AND vancomycin undosable (renal failure +
    # nephrotoxin) → no safe empiric regimen → honest abstention, not a fabricated one.
    assert "betalactam_allergy_severe" in r["exclusions"]
    assert "vancomycin" in r["dose_infeasible"]
    assert r["regimen"] is None and r["outcome"] == "infeasible" and r["conflict"] is not None
    print("test_deidentify: PASS (no PHI survives; clinical signal preserved; idempotent; "
          "chart→ChartFacts mapped; de-identified complex chart → optimizer abstains; 0 model calls)")


def main() -> int:
    test_no_phi_survives()
    test_clinical_signal_preserved()
    test_deidentify_does_not_mutate_input_and_is_idempotent()
    test_residual_phi_paths_closed()
    test_age_over_89_is_capped()
    test_chart_to_chartfacts_mapping()
    test_end_to_end_complex_chart_abstains()
    return 0


if __name__ == "__main__":
    sys.exit(main())
