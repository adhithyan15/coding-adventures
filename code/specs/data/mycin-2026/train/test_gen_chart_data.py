#!/usr/bin/env python3
"""test_gen_chart_data.py — guard the chart-fact decomposer data generator (F3).

Pure checks (no teacher / Ollama / MLX). The headline is the F3→F2 CONSUMABILITY
contract: every (kind, value) the generator can sample is mapped by the chart-as-
constraints compiler (`chart_to_cop.compile_cop`) — i.e. the decomposer's gold IR can
never contain a chart fact the COP would silently discard. Plus: the gold carries
verbatim byte-provenance spans, an unstated fact degrades to type:inferred, distractors
become justified discards, and the abstain case yields no chart facts.
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
sys.path.insert(0, str(MYCIN / "treatment" / "antibiotics"))
import chart_to_cop as cc  # noqa: E402  (the F2 consumer — closed-vocab gate)
import gen_chart_data as gcd  # noqa: E402


def test_every_sampled_chart_fact_is_consumable_by_the_cop() -> None:
    # The F3→F2 contract: each (kind, value) the decomposer may emit must compile into a
    # COP input, never land in compile_cop's discards (which would mean an unmapped fact).
    for kind, options in gcd.CHART_PROFILES.items():
        for value, surfaces in options:
            cop = cc.compile_cop([cc.ChartFact(kind, value, surfaces[0])])
            unmapped = [d for d in cop.discards if d["fact"].startswith(f"{kind}=")]
            assert not unmapped, f"{kind}={value} is discarded by compile_cop: {unmapped}"


def test_sampled_charts_stay_in_vocab_and_include_an_age_band() -> None:
    for seed in range(40):
        facts = gcd.sample_chart(random.Random(seed))
        if not facts:
            continue  # abstain case
        assert facts[0]["kind"] == "age_band", facts
        for f in facts:
            assert f["kind"] in gcd.CHART_PROFILES
            vals = {v for v, _ in gcd.CHART_PROFILES[f["kind"]]}
            assert f["value"] in vals, f


def test_build_gold_grounds_stated_facts_with_spans() -> None:
    note = ("A 72-year-old man, post-operative day 3 from a craniotomy, with an eGFR of 12; "
            "he drove himself to the emergency department.")
    facts = [{"kind": "age_band", "value": "older_adult", "surfaces": ["a 72-year-old"]},
             {"kind": "setting", "value": "post_neurosurgical",
              "surfaces": ["post-operative day 3 from a craniotomy"]},
             {"kind": "renal_status", "value": "renal_severe", "surfaces": ["an eGFR of 12"]}]
    distractors = [("drove himself to the emergency department", "logistics, not a chart fact")]
    gold = gcd.build_gold_chart_ir(note, facts, distractors)
    assert len(gold["chart_facts"]) == 3
    for gf in gold["chart_facts"]:
        for k in ("kind", "value", "span", "type"):
            assert k in gf, gf
        assert gf["span"] and gf["span"].lower() in note.lower(), gf
        assert gf["type"] == "stated"
    # the distractor that appears in the note is recorded as a justified discard
    assert len(gold["discard"]) == 1 and gold["discard"][0]["span"].lower() in note.lower()
    assert gold["discard"][0]["reason"]
    # and the gold facts are consumable by the COP (end-to-end F3→F2 on a real note)
    cop = cc.compile_cop([cc.ChartFact(gf["kind"], gf["value"], gf["span"])
                          for gf in gold["chart_facts"]])
    assert "post_neurosurgical" in cop.organisms or cop.organisms  # scenario selected
    assert not any(d["fact"].split("=")[0] in {"age_band", "setting", "renal_status"}
                   for d in cop.discards)


def test_unstated_fact_degrades_to_inferred_empty_span() -> None:
    # The note does NOT mention pregnancy → the fact can only be a non-verbatim inference.
    note = "A 45-year-old man with a headache."
    facts = [{"kind": "age_band", "value": "adult", "surfaces": ["a 45-year-old man"]},
             {"kind": "pregnancy", "value": "present", "surfaces": ["28 weeks pregnant"]}]
    gold = gcd.build_gold_chart_ir(note, facts, [])
    by_kind = {gf["kind"]: gf for gf in gold["chart_facts"]}
    assert by_kind["age_band"]["span"] and by_kind["age_band"]["type"] == "stated"
    assert by_kind["pregnancy"]["span"] == "" and by_kind["pregnancy"]["type"] == "inferred"


def test_prompt_lists_the_closed_vocabulary() -> None:
    p = gcd.prompt_for_chart("A 50-year-old with fever.")
    for kind in ("age_band", "allergy", "renal_status", "culture_resistance"):
        assert kind in p, kind
    assert "discard" in p and "span" in p


def test_distractor_pool_well_formed() -> None:
    assert gcd.DISTRACTORS and gcd.NEAR_MISS_DISTRACTORS
    for phrase, reason in gcd.DISTRACTORS + gcd.NEAR_MISS_DISTRACTORS:
        assert isinstance(phrase, str) and len(phrase) >= 4 and isinstance(reason, str) and reason


def test_near_miss_distractors_are_discarded_never_coined() -> None:
    # A near-miss look-alike present in a note (with NO matching sampled fact) must be recorded
    # as a justified discard and must NOT become a chart fact — only sampled facts coin facts, so
    # the gold is correct by construction; this pins that contract over the whole near-miss pool.
    for phrase, reason in gcd.NEAR_MISS_DISTRACTORS:
        note = f"A 50-year-old patient with a headache; {phrase}."
        facts = [{"kind": "age_band", "value": "adult", "surfaces": ["A 50-year-old"]}]
        gold = gcd.build_gold_chart_ir(note, facts, [(phrase, reason)])
        # exactly the one sampled fact (age_band) — the near-miss did NOT coin a look-alike fact.
        assert [gf["kind"] for gf in gold["chart_facts"]] == ["age_band"], (phrase, gold)
        # and the near-miss is set aside with its reason, span verbatim in the note.
        assert len(gold["discard"]) == 1, gold
        assert gold["discard"][0]["span"].lower() in note.lower()
        assert gold["discard"][0]["reason"] == reason


def test_near_miss_does_not_add_a_second_lookalike_fact() -> None:
    # Discrimination: the SAME drug name ("penicillin") appears as a real allergy AND as an
    # efficacy near-miss in one note. The allergy phrasing coins exactly one allergy fact; the
    # efficacy phrasing is discarded — never a second, spurious allergy.
    note = ("A 45-year-old man with anaphylaxis to penicillin; note that penicillin cleared "
            "her last urinary infection.")
    facts = [{"kind": "age_band", "value": "adult", "surfaces": ["A 45-year-old man"]},
             {"kind": "allergy", "value": "penicillin", "surfaces": ["anaphylaxis to penicillin"]}]
    near = ("penicillin cleared her last urinary infection",
            "drug EFFICACY history, NOT an allergy — wrong relation")
    gold = gcd.build_gold_chart_ir(note, facts, [near])
    allergy_facts = [gf for gf in gold["chart_facts"] if gf["kind"] == "allergy"]
    assert len(allergy_facts) == 1 and allergy_facts[0]["value"] == "penicillin", gold
    assert allergy_facts[0]["span"] == "anaphylaxis to penicillin", gold
    assert any("urinary infection" in d["span"] for d in gold["discard"]), gold


def main() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"  FAIL  {t.__name__}: {exc}")
    print(f"\ntest_gen_chart_data: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
