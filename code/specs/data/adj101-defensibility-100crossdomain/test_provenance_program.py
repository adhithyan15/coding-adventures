#!/usr/bin/env python3
"""ADJ101 — tests for the provenance-gated executor. The gates must BITE on violations."""
import provenance_program as P

SPANS = ["20 m/s", "30 degrees", "9.8 m/s^2", "2 meters wide"]

FACTS = {
    "v0": {"magnitude": 20, "unit": "m/s", "type": "stated", "span": "20 m/s"},
    "angle_deg": {"magnitude": 30, "unit": "degree", "type": "stated", "span": "30 degrees"},
    "g": {"magnitude": 9.8, "unit": "m/s^2", "type": "stated", "span": "9.8 m/s^2"},
}
PROG_OK = ("import sympy as sp\n"
           "RESULT = float((facts['v0']['magnitude']*sp.sin(sp.rad(facts['angle_deg']['magnitude'])))**2"
           "/(2*facts['g']['magnitude']))\n")
DISCARD = [{"span": "2 meters wide", "reason": "pad width irrelevant to vertical max height"}]


def test_clean_passes():
    out = P.adjudicate_program(SPANS, {"facts": FACTS, "discarded": DISCARD, "program": PROG_OK}, 5.102, 0.02)
    assert out["provenance_clean"] and out["correct"], out


def test_dropped_distractor_flagged():
    # distractor "2 meters wide" neither used nor discarded -> coverage gate must flag the phrase
    out = P.adjudicate_program(SPANS, {"facts": FACTS, "discarded": [], "program": PROG_OK}, 5.102, 0.02)
    assert "2 meters wide" in out["missing_coverage"], out["missing_coverage"]
    assert not out["provenance_clean"], out


def test_magic_number_flagged():
    # program hard-codes 9.8 instead of facts['g'] -> magic-number gate must flag 9.8
    prog_magic = ("import sympy as sp\n"
                  "RESULT = float((facts['v0']['magnitude']*sp.sin(sp.rad(facts['angle_deg']['magnitude'])))**2/(2*9.8))\n")
    out = P.adjudicate_program(SPANS, {"facts": FACTS, "discarded": DISCARD, "program": prog_magic}, 5.102, 0.02)
    assert 9.8 in out["magic_numbers"], out["magic_numbers"]
    assert not out["provenance_clean"], out


def test_fabricated_fact_flagged():
    # a fact with no provenance (no span, no basis) -> justification gate must flag it
    facts_bad = {**FACTS, "made_up": {"magnitude": 7, "unit": "m", "type": "inferred"}}
    out = P.adjudicate_program(SPANS, {"facts": facts_bad, "discarded": DISCARD, "program": PROG_OK}, 5.102, 0.02)
    assert "made_up" in out["fabrications"], out["fabrications"]
    assert not out["provenance_clean"], out


def test_inferred_with_entailment_passes_justification():
    facts_inf = {**FACTS, "half_g": {"magnitude": 4.9, "type": "inferred",
                                     "basis_span": "9.8 m/s^2", "entailment": "ENTAILED"}}
    assert P.check_justification(facts_inf) == []  # entailed + basis -> justified


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in fns:
        fn()
        print("PASS", fn.__name__)
    print(f"\nall {len(fns)} tests passed")
