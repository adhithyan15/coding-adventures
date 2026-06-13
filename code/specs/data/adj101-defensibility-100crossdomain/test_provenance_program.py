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
    assert out["auditable"] and out["correct"], out


def test_dropped_distractor_flagged():
    # distractor "2 meters wide" neither used nor discarded -> coverage gate must flag the phrase
    out = P.adjudicate_program(SPANS, {"facts": FACTS, "discarded": [], "program": PROG_OK}, 5.102, 0.02)
    assert "2 meters wide" in out["missing_coverage"], out["missing_coverage"]
    assert not out["auditable"], out


def test_magic_number_flagged():
    # program hard-codes 9.8 instead of facts['g'] -> magic-number gate must flag 9.8
    prog_magic = ("import sympy as sp\n"
                  "RESULT = float((facts['v0']['magnitude']*sp.sin(sp.rad(facts['angle_deg']['magnitude'])))**2/(2*9.8))\n")
    out = P.adjudicate_program(SPANS, {"facts": FACTS, "discarded": DISCARD, "program": prog_magic}, 5.102, 0.02)
    assert 9.8 in out["magic_numbers"], out["magic_numbers"]
    assert not out["auditable"], out


def test_fabricated_fact_flagged():
    # a fact with no provenance (no span, no basis) -> justification gate must flag it
    facts_bad = {**FACTS, "made_up": {"magnitude": 7, "unit": "m", "type": "inferred"}}
    out = P.adjudicate_program(SPANS, {"facts": facts_bad, "discarded": DISCARD, "program": PROG_OK}, 5.102, 0.02)
    assert "made_up" in out["fabrications"], out["fabrications"]
    assert not out["auditable"], out


def test_inferred_with_entailment_passes_justification():
    facts_inf = {**FACTS, "half_g": {"magnitude": 4.9, "type": "inferred",
                                     "basis_span": "9.8 m/s^2", "entailment": "ENTAILED"}}
    j = P.check_justification(facts_inf)
    assert j["fabrications"] == [] and j["surfaced_assumptions"] == []  # entailed + basis -> grounded


def test_leap_inference_is_surfaced_assumption_not_fabrication():
    facts_leap = {**FACTS, "ratio": {"magnitude": 1, "type": "inferred",
                                     "basis_span": "carbon dioxide produced", "entailment": "LEAP"}}
    j = P.check_justification(facts_leap)
    assert j["fabrications"] == [] and j["surfaced_assumptions"] == ["ratio"]  # auditable, not fabricated


def test_unit_converted_value_is_faithful():
    # 4% typed as the fraction 0.04 must NOT be flagged unfaithful (the IR is unit-typed)
    assert P.check_faithfulness({"rate": {"magnitude": 0.04, "type": "stated", "span": "4%"}}) == []


def test_non_numeric_datum_not_flagged_unfaithful():
    # a SMILES string fact is data, not a quantity -> not flagged when the string is in the span
    assert P.check_faithfulness({"smiles": {"magnitude": "O=C=O", "type": "stated", "span": "SMILES O=C=O"}}) == []


# --- the rescored paradigm for programs: WRONG is fine if it's localized + correctable -------------

# A mis-extraction: v0 read as 25 but its cited span still says "20 m/s" (value contradicts the bytes).
WRONG = {
    "facts": {**FACTS, "v0": {"magnitude": 25, "unit": "m/s", "type": "stated", "span": "20 m/s"}},
    "discarded": DISCARD, "program": PROG_OK,
}


def test_wrong_answer_is_localized_to_the_exact_fact():
    out = P.adjudicate_program(SPANS, WRONG, 5.102, 0.02)
    assert out["correct"] is False, out["result"]          # it got the wrong answer ...
    assert out["unfaithful_facts"] == ["v0"], out          # ... and the audit points EXACTLY at v0
    assert out["error_locus"]["unfaithful_facts"] == ["v0"]
    # correctness is not the target; localization is. The trail names the one thing to fix.


def test_override_the_fact_re_derives_correctly_with_no_model_call():
    out_wrong = P.adjudicate_program(SPANS, WRONG, 5.102, 0.02)
    assert out_wrong["correct"] is False
    fixed = P.override_facts(WRONG, {"v0": 20})             # fix the fact, not the weight
    out_fixed = P.adjudicate_program(SPANS, fixed, 5.102, 0.02)
    assert out_fixed["correct"] is True and out_fixed["auditable"], out_fixed
    assert fixed["facts"]["v0"]["_override"] == {"from": 25, "to": 20}  # the correction is itself audited


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in fns:
        fn()
        print("PASS", fn.__name__)
    print(f"\nall {len(fns)} tests passed")
