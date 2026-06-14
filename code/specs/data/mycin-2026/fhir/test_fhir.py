#!/usr/bin/env python3
"""test_fhir.py - guard FHIR ingestion: coded chart -> typed findings, deterministic.

The coded path needs NO model: a FHIR Bundle maps to typed findings by a pure
LOINC/SNOMED + interpretation lookup. Tests cover the mapping rules, robustness to
messy/empty resources, that unknown codes are surfaced (never guessed), and the
full chart -> diagnosis when the CLI is built. CI runs all of this.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402
import fhir_ingest as fhir  # noqa: E402

BUNDLE = HERE / "samples" / "meningitis_bundle.json"


def _obs(code: str, interp: str | None = None, qty: dict | None = None) -> dict:
    o = {"resourceType": "Observation",
         "code": {"coding": [{"system": "http://loinc.org", "code": code}]}}
    if interp:
        o["interpretation"] = [{"coding": [
            {"system": fhir.INTERP_SYSTEM, "code": interp}]}]
    if qty:
        o["valueQuantity"] = qty
    return o


def test_observation_interpretation_mapping() -> None:
    assert fhir.observation_to_finding(_obs("2342-4", "L")) == ("csf_glucose", "low")
    assert fhir.observation_to_finding(_obs("2880-3", "H")) == ("csf_protein", "high")
    assert fhir.observation_to_finding(_obs("664-3", "POS")) == ("csf_gram_stain", "positive")
    assert fhir.observation_to_finding(_obs("600-7", "NEG")) == ("csf_culture", "negative")


def test_temperature_threshold_and_fahrenheit() -> None:
    assert fhir.observation_to_finding(_obs("8310-5", qty={"value": 39.1, "unit": "Cel"})) == ("fever", "present")
    assert fhir.observation_to_finding(_obs("8310-5", qty={"value": 37.0, "unit": "Cel"})) == ("fever", "absent")
    # Fahrenheit is normalized: 102.4 F = 39.1 C -> present.
    assert fhir.observation_to_finding(_obs("8310-5", qty={"value": 102.4, "unit": "[degF]"})) == ("fever", "present")


def test_unknown_code_is_not_guessed() -> None:
    assert fhir.observation_to_finding(_obs("99999-9", "H")) is None
    assert fhir.condition_to_finding({"code": {"coding": [
        {"system": fhir.CODE_MAP["snomed_system"], "code": "00000000"}]}}) is None


def test_condition_mapping() -> None:
    cond = {"code": {"coding": [{"system": fhir.CODE_MAP["snomed_system"], "code": "161115002"}]}}
    assert fhir.condition_to_finding(cond) == ("meningismus", "present")


def test_extract_is_robust_to_empty_and_messy() -> None:
    assert fhir.extract({}) ["findings"] == []
    assert fhir.extract({"entry": [{"resource": {"resourceType": "Observation"}}]})["unmapped"]


def test_extract_sample_bundle() -> None:
    chart = fhir.extract(__import__("json").loads(BUNDLE.read_text()))
    fs = set(chart["findings"])
    assert {"csf_glucose(low)", "csf_protein(high)", "csf_neutrophilic_pleocytosis(high)",
            "csf_gram_stain(positive)", "fever(present)", "meningismus(present)"} <= fs, fs
    assert "Penicillin" in chart["allergies"]
    assert chart["demographics"]["gender"] == "male"


def main() -> int:
    test_observation_interpretation_mapping()
    test_temperature_threshold_and_fahrenheit()
    test_unknown_code_is_not_guessed()
    test_condition_mapping()
    test_extract_is_robust_to_empty_and_messy()
    test_extract_sample_bundle()

    # Full coded-chart -> diagnosis, 0 model calls (only if the CLI is built).
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_fhir: PASS (ingestion + mapping); CLI checks SKIPPED (adj-lang-cli not built)")
        return 0
    import ir_to_adj as ir_mod
    chart = fhir.extract(__import__("json").loads(BUNDLE.read_text()))
    ir = {"case_id": "fhir", "findings": [{"term": t, "type": "stated", "polarity": "affirmed"}
                                          for t in chart["findings"]],
          "discard": [], "inference_justifications": []}
    observe_adj, kept, _ = ir_mod.ir_to_adj(ir, ir_mod.load_domains())
    res = decide_mod.decide("fhir", observe_adj, cli)
    assert res["leader"] == "bacterial_meningitis", res
    print("test_fhir: PASS (coded FHIR Bundle -> typed findings -> bacterial_meningitis; "
          "unknown codes surfaced not guessed; 0 model calls)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
