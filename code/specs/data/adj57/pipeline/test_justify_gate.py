#!/usr/bin/env python3
"""Tests for the justification gate (justify_gate.py). Run: python test_justify_gate.py

Covers LAYER 1 (byte-anchor: every cited span verbatim) and the aggregation of LAYER 2
(the justification verdict), across the evidence/conclusion split.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import justify_gate  # noqa: E402

INPUT = ["hepatomegaly of 3 cm and splenomegaly of 3 cm. Cultures from blood and urine were sterile. travel through Uganda", "hepatosplenomegaly", "blood_urine_cultures_sterile"]


def g(**kw):
    kw.setdefault("kind", "evidence")
    kw.setdefault("justified", True)
    return kw


# ---- LAYER 1: byte-anchor ----
def test_all_spans_must_be_verbatim():
    a = justify_gate.anchor(INPUT, ["hepatomegaly of 3 cm", "NOT IN INPUT"])
    assert not a["anchored"] and a["missing"] == ["NOT IN INPUT"]  # one fabricated cite fails the whole claim


def test_anchored_when_every_span_present():
    a = justify_gate.anchor(INPUT, ["hepatomegaly of 3 cm", "hepatosplenomegaly"])
    assert a["anchored"] and not a["missing"]


def test_no_citation_is_not_anchored():
    assert not justify_gate.anchor(INPUT, [])["anchored"]


# ---- combine multiple bytes into one justified fact ----
def test_combination_of_bytes_grounds_a_synthesis():
    r = justify_gate.grade(INPUT, [g(
        claim="disseminated infection with reticuloendothelial involvement",
        grounded_by=["hepatomegaly of 3 cm and splenomegaly of 3 cm", "sterile"], justified=True)])
    assert r["fully_grounded"] and r["n_grounded"] == 1


# ---- LAYER 2 verdict is respected ----
def test_anchored_but_unjustified_is_rejected():
    r = justify_gate.grade(INPUT, [g(claim="it is tremolitized", grounded_by=["sterile"], justified=False)])
    assert not r["fully_grounded"] and "do NOT justify" in r["rejected"][0]["reason"]


def test_conclusion_justified_is_grounded():
    r = justify_gate.grade(INPUT, [g(
        claim="neurobrucellosis is the most likely diagnosis", kind="conclusion",
        grounded_by=["travel through Uganda", "blood_urine_cultures_sterile"], justified=True)])
    assert r["fully_grounded"] and r["n_conclusion"] == 1


def test_conclusion_unjustified_is_rejected():
    r = justify_gate.grade(INPUT, [g(
        claim="this is confirmed malaria", kind="conclusion",
        grounded_by=["travel through Uganda"], justified=False)])
    assert not r["fully_grounded"] and "not warranted" in r["rejected"][0]["reason"]


def test_fabricated_citation_beats_a_true_verdict():
    # even if the verifier says justified, a non-verbatim citation fails the byte-anchor
    r = justify_gate.grade(INPUT, [g(claim="x", grounded_by=["ghost span"], justified=True)])
    assert not r["fully_grounded"] and "fabricated" in r["rejected"][0]["reason"]


def test_unknown_kind_rejected():
    r = justify_gate.grade(INPUT, [g(claim="x", kind="guess", grounded_by=["sterile"], justified=True)])
    assert not r["fully_grounded"] and "unknown claim kind" in r["rejected"][0]["reason"]


def test_mixed_evidence_and_conclusion_counts():
    r = justify_gate.grade(INPUT, [
        g(claim="cultures sterile", grounded_by=["sterile"], justified=True),
        g(claim="zoonosis likely", kind="conclusion", grounded_by=["travel through Uganda"], justified=True),
        g(claim="invented", grounded_by=["not here"], justified=True)])
    assert r["n_grounded"] == 2 and r["n_rejected"] == 1 and r["n_evidence"] == 1 and r["n_conclusion"] == 1
    assert not r["fully_grounded"]


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in fns:
        fn()
        print(f"  ok  {fn.__name__}")
    print(f"\n{len(fns)} tests passed.")
