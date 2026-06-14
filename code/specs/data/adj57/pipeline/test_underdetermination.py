#!/usr/bin/env python3
"""Tests for the underdetermination gate (underdetermination.py).
Run: python test_underdetermination.py"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import underdetermination as u  # noqa: E402

INPUT = ["Cultures were sterile. The chemical composition conforms to EA1N. Beach marks are present."]


def test_determined_when_every_rival_has_a_present_cited_datum():
    r = u.assess(INPUT, [
        {"hypothesis": "material defect", "discriminating_observation": "chemistry conformance",
         "present": True, "citation": "The chemical composition conforms to EA1N"}])
    assert r["determined"] and r["n_open"] == 0 and not r["holes"]


def test_underdetermined_when_discriminating_datum_absent():
    # the observation that would rule out the rival is simply not in the bytes
    r = u.assess(INPUT, [
        {"hypothesis": "operating stress exceeded fatigue limit",
         "discriminating_observation": "operating stress vs fatigue limit comparison",
         "present": False, "citation": ""}])
    assert not r["determined"] and r["n_open"] == 1
    assert r["holes"] == ["operating stress vs fatigue limit comparison"]
    assert "ABSENT" in r["open"][0]["why"]


def test_claimed_present_but_fabricated_citation_is_open():
    # cannot CLAIM the datum is present without a verbatim byte
    r = u.assess(INPUT, [
        {"hypothesis": "overstress", "discriminating_observation": "FEA stress 298 MPa",
         "present": True, "citation": "FEA shows 298 MPa"}])  # not in input
    assert not r["determined"] and r["n_open"] == 1
    assert "not verbatim" in r["open"][0]["why"]


def test_mixed_resolved_and_open():
    r = u.assess(INPUT, [
        {"hypothesis": "material defect", "discriminating_observation": "chemistry",
         "present": True, "citation": "conforms to EA1N"},
        {"hypothesis": "overstress", "discriminating_observation": "stress vs fatigue limit",
         "present": False, "citation": ""}])
    assert r["n_resolved"] == 1 and r["n_open"] == 1 and not r["determined"]
    assert r["holes"] == ["stress vs fatigue limit"]


def test_no_rivals_is_trivially_determined():
    # nothing competes with the conclusion -> determined (vacuously)
    r = u.assess(INPUT, [])
    assert r["determined"] and r["n_open"] == 0


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in fns:
        fn()
        print(f"  ok  {fn.__name__}")
    print(f"\n{len(fns)} tests passed.")
