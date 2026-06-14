#!/usr/bin/env python3
"""Tests for the output-grounding gate (output_gate.py). Run: python test_output_gate.py"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import output_gate  # noqa: E402

INPUT = ["The rock has RI 1.57, SG 2.77, well-developed cleavage.", "rock(white_basal_mineral)"]


def test_claim_with_verbatim_citation_is_grounded():
    r = output_gate.ground_output(INPUT, [{"claim": "base is calcic plagioclase", "grounded_by": ["RI 1.57", "SG 2.77"]}])
    assert r["fully_grounded"] and r["n_grounded"] == 1


def test_claim_citing_absent_span_is_ungrounded():
    r = output_gate.ground_output(INPUT, [{"claim": "it is tremolitized", "grounded_by": ["tremolite"]}])  # not in input
    assert not r["fully_grounded"] and r["n_ungrounded"] == 1
    assert "tremolitized" in r["ungrounded"][0]["claim"]


def test_claim_with_no_citation_is_ungrounded():
    r = output_gate.ground_output(INPUT, [{"claim": "it is from Mars", "grounded_by": []}])
    assert not r["fully_grounded"]
    assert "NO citation" in r["ungrounded"][0]["reason"]


def test_one_good_citation_is_enough():
    r = output_gate.ground_output(INPUT, [{"claim": "mixed", "grounded_by": ["NOT THERE", "RI 1.57"]}])
    assert r["fully_grounded"]  # at least one citation retrieves


def test_can_cite_a_fact_term():
    r = output_gate.ground_output(INPUT, [{"claim": "white base", "grounded_by": ["rock(white_basal_mineral)"]}])
    assert r["fully_grounded"]


def test_mixed_set_partial():
    r = output_gate.ground_output(INPUT, [
        {"claim": "grounded one", "grounded_by": ["cleavage"]},
        {"claim": "invented one", "grounded_by": ["zircon"]}])
    assert not r["fully_grounded"] and r["n_grounded"] == 1 and r["n_ungrounded"] == 1


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in fns:
        fn()
        print(f"  ok  {fn.__name__}")
    print(f"\n{len(fns)} tests passed.")
