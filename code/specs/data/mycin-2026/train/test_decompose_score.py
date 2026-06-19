#!/usr/bin/env python3
"""test_decompose_score.py — guard the model-free decompose-fidelity scorer.

Pure unit tests (no model/network): feed synthetic predicted IRs against a fixed gold + note and
assert each metric responds correctly — a perfect prediction scores 1.0 across the board; a miss,
an over-extraction, a hallucinated span, a near-miss-coined-as-fact, and an honest abstain each
move exactly the metric they should. Covers both IR shapes (chart-facts + findings).
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import decompose_score as ds  # noqa: E402

NOTE = ("A 72-year-old man with an eGFR of 12; his father has chronic kidney disease. "
        "He drove himself to the emergency department.")

GOLD = {
    "chart_facts": [
        {"kind": "age_band", "value": "older_adult", "span": "A 72-year-old", "type": "stated"},
        {"kind": "renal_status", "value": "renal_severe", "span": "an eGFR of 12", "type": "stated"},
    ],
    "discard": [
        {"span": "his father has chronic kidney disease",
         "reason": "family history (renal), NOT the patient's renal_status"},
        {"span": "drove himself to the emergency department", "reason": "logistics"},
    ],
}


def test_perfect_prediction_scores_one():
    s = ds.score_decompose(GOLD, GOLD, NOTE, ds.CHART_FACTS)
    assert s["fact_precision"] == 1.0 and s["fact_recall"] == 1.0 and s["fact_f1"] == 1.0
    assert s["span_faithfulness"] == 1.0
    assert s["discard_recall"] == 1.0 and s["discard_precision"] == 1.0
    assert s["near_miss_violations"] == 0 and s["false_positive_facts"] == 0


def test_missing_a_fact_drops_recall_only():
    pred = {"chart_facts": GOLD["chart_facts"][:1], "discard": GOLD["discard"]}  # drop renal
    s = ds.score_decompose(pred, GOLD, NOTE, ds.CHART_FACTS)
    assert s["fact_precision"] == 1.0  # what it did emit is correct
    assert s["fact_recall"] == 0.5  # 1 of 2 gold facts
    assert s["false_positive_facts"] == 0


def test_over_extraction_drops_precision():
    pred = {"chart_facts": GOLD["chart_facts"] + [
        {"kind": "pregnancy", "value": "present", "span": "", "type": "inferred"}], "discard": []}
    s = ds.score_decompose(pred, GOLD, NOTE, ds.CHART_FACTS)
    assert s["fact_recall"] == 1.0
    assert s["fact_precision"] < 1.0  # 2 of 3 predicted match gold
    assert s["false_positive_facts"] == 1


def test_hallucinated_span_drops_faithfulness():
    pred = {"chart_facts": [
        {"kind": "age_band", "value": "older_adult", "span": "A 72-year-old", "type": "stated"},
        {"kind": "renal_status", "value": "renal_severe",
         "span": "dialysis three times a week", "type": "stated"}],  # span NOT in the note
        "discard": GOLD["discard"]}
    s = ds.score_decompose(pred, GOLD, NOTE, ds.CHART_FACTS)
    assert s["fact_recall"] == 1.0 and s["fact_precision"] == 1.0  # identity keys still match
    assert s["span_faithfulness"] == 0.5  # 1 of 2 cited spans is verbatim in the note


def test_near_miss_coined_as_fact_is_flagged():
    # The model wrongly coined a renal_status fact from the FATHER's CKD (a gold discard span).
    pred = {"chart_facts": GOLD["chart_facts"] + [
        {"kind": "renal_status", "value": "renal_moderate",
         "span": "his father has chronic kidney disease", "type": "stated"}],
        "discard": []}
    s = ds.score_decompose(pred, GOLD, NOTE, ds.CHART_FACTS)
    assert s["near_miss_violations"] == 1, s  # the headline faithfulness failure is caught
    assert s["false_positive_facts"] == 1  # it's also a false positive (not in gold)


def test_missing_a_discard_drops_discard_recall():
    pred = {"chart_facts": GOLD["chart_facts"], "discard": GOLD["discard"][:1]}  # drop one discard
    s = ds.score_decompose(pred, GOLD, NOTE, ds.CHART_FACTS)
    assert s["discard_recall"] == 0.5 and s["discard_precision"] == 1.0


def test_honest_abstain_scores_perfectly():
    note = "A 50-year-old with a headache and nothing else of note."
    empty_gold = {"chart_facts": [], "discard": []}
    s = ds.score_decompose(empty_gold, empty_gold, note, ds.CHART_FACTS)
    assert s["fact_precision"] == 1.0 and s["fact_recall"] == 1.0 and s["fact_f1"] == 1.0
    assert s["near_miss_violations"] == 0 and s["false_positive_facts"] == 0


def test_findings_shape_keys_on_polarity():
    note = "The CSF Gram stain was negative; he has a fever."
    gold = {"findings": [
        {"functor": "csf_gram_stain", "value": "negative", "polarity": "denied",
         "span": "Gram stain was negative", "type": "stated"},
        {"functor": "fever", "value": "present", "polarity": "affirmed",
         "span": "a fever", "type": "stated"}], "discard": []}
    # A prediction that flips the gram-stain polarity to affirmed must NOT match (wrong polarity).
    pred = {"findings": [
        {"functor": "csf_gram_stain", "value": "negative", "polarity": "affirmed",
         "span": "Gram stain was negative", "type": "stated"},
        {"functor": "fever", "value": "present", "polarity": "affirmed",
         "span": "a fever", "type": "stated"}], "discard": []}
    s = ds.score_decompose(pred, gold, note, ds.FINDINGS)
    assert s["fact_recall"] == 0.5, s  # only fever matches; flipped-polarity gram stain does not
    assert s["false_positive_facts"] == 1


def test_aggregate_means_ratios_and_sums_counts():
    perfect = ds.score_decompose(GOLD, GOLD, NOTE, ds.CHART_FACTS)
    bad = ds.score_decompose(
        {"chart_facts": GOLD["chart_facts"] + [
            {"kind": "renal_status", "value": "renal_moderate",
             "span": "his father has chronic kidney disease", "type": "stated"}], "discard": []},
        GOLD, NOTE, ds.CHART_FACTS)
    agg = ds.aggregate([perfect, bad])
    assert agg["n"] == 2
    assert agg["near_miss_violations"] == 1  # summed (0 + 1)
    assert 0.0 < agg["fact_precision"] < 1.0  # mean of 1.0 and <1.0
    assert ds.aggregate([]) == {}


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
    print(f"\ntest_decompose_score: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
