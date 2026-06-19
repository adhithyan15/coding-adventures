#!/usr/bin/env python3
"""test_eval_decompose.py — guard the held-out decompose eval set + runner (offline, no model).

Two guarantees: (1) the curated gold is SELF-CONSISTENT — every cited span is verbatim in its
note, every chart-fact (kind,value) is COP-consumable, every near-miss is a discard (never a
gold fact), every findings term agrees with functor/value; (2) the runner + scorer COMPOSE —
scoring the gold as the prediction (`--self-check`) is a perfect 1.0 with zero violations.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "treatment" / "antibiotics"))
import chart_to_cop as cc  # noqa: E402  (the COP consumer — closed-vocab gate)
import eval_decompose as ed  # noqa: E402

RECORDS = ed.load_eval()


def test_eval_set_is_nonempty_and_well_shaped():
    assert RECORDS, "the held-out eval set must not be empty"
    ids = [r["id"] for r in RECORDS]
    assert len(ids) == len(set(ids)), "eval ids must be unique"
    for r in RECORDS:
        assert r["shape"] in ed.SHAPES, r
        items_field = ed.SHAPES[r["shape"]][0]
        assert items_field in r["gold"] and "discard" in r["gold"], r
    # both shapes are represented, and the near-miss/abstain cases are present.
    assert {r["shape"] for r in RECORDS} == {"chart_facts", "findings"}
    assert any(not r["gold"][ed.SHAPES[r["shape"]][0]] for r in RECORDS), "need an abstain case"
    assert any(r["gold"]["discard"] for r in RECORDS), "need a near-miss/discard case"


def test_every_gold_span_is_verbatim_in_its_note():
    # Byte-provenance: each cited span (facts + discards) must be a verbatim substring of the note.
    for r in RECORDS:
        note = ed.ds._norm(r["note"])
        items_field = ed.SHAPES[r["shape"]][0]
        for item in r["gold"][items_field]:
            if item.get("span"):
                assert ed.ds._norm(item["span"]) in note, (r["id"], item)
        for d in r["gold"]["discard"]:
            assert ed.ds._norm(d["span"]) in note, (r["id"], d)
            assert d.get("reason"), (r["id"], d)


def test_every_chart_fact_gold_is_cop_consumable():
    # Closed-vocab: each chart-fact (kind,value) must compile into a COP input, never discarded.
    for r in RECORDS:
        if r["shape"] != "chart_facts":
            continue
        for f in r["gold"]["chart_facts"]:
            cop = cc.compile_cop([cc.ChartFact(f["kind"], f["value"], f["span"] or f["value"])])
            unmapped = [d for d in cop.discards if d["fact"].startswith(f'{f["kind"]}=')]
            assert not unmapped, (r["id"], f, unmapped)


def test_findings_terms_agree_with_functor_value():
    for r in RECORDS:
        if r["shape"] != "findings":
            continue
        for f in r["gold"]["findings"]:
            assert f["term"] == f'{f["functor"]}({f["value"]})', (r["id"], f)
            assert f["polarity"] in ("affirmed", "denied"), (r["id"], f)


def test_self_check_scores_perfect():
    # Scoring the gold as the prediction must be a flawless pass — the set + scorer compose, and
    # there is no hidden near-miss violation or unverifiable span lurking in the curated gold.
    predictions = {r["id"]: r["gold"] for r in RECORDS}
    rows, agg = ed.score_predictions(RECORDS, predictions)
    assert agg["n"] == len(RECORDS)
    assert agg["fact_f1"] == 1.0 and agg["span_faithfulness"] == 1.0
    assert agg["discard_recall"] == 1.0 and agg["discard_precision"] == 1.0
    assert agg["near_miss_violations"] == 0 and agg["false_positive_facts"] == 0
    for row in rows:  # every individual example is perfect too
        assert row["fact_f1"] == 1.0 and row["near_miss_violations"] == 0, row


def test_a_near_miss_prediction_is_caught_by_the_eval():
    # Sanity that the eval has teeth: a prediction that coins a fact from a near-miss discard span
    # (the family-history CKD) is scored as a near_miss_violation against the curated gold.
    rec = next(r for r in RECORDS if r["id"] == "cf_family_history_nearmiss")
    bad_pred = {"chart_facts": rec["gold"]["chart_facts"] + [
        {"kind": "renal_status", "value": "renal_moderate",
         "span": "his father has chronic kidney disease", "type": "stated"}], "discard": []}
    s = ed.ds.score_decompose(bad_pred, rec["gold"], rec["note"], ed.ds.CHART_FACTS)
    assert s["near_miss_violations"] == 1 and s["false_positive_facts"] == 1, s


def test_parse_ir_extracts_json_from_noisy_output():
    # The model may wrap its JSON in prose / code fences; parse_ir pulls the first object out.
    assert ed.parse_ir('Sure! ```json\n{"chart_facts": [], "discard": []}\n``` done')["chart_facts"] == []
    assert ed.parse_ir("no json here") == {}  # garbage → empty IR (penalized, never crashes)
    assert ed.parse_ir('{"findings": [{"functor": "fever"}]}')["findings"][0]["functor"] == "fever"


def test_model_mode_wiring_scores_a_stub_generator_perfectly():
    # The --model path is exercised end-to-end WITHOUT MLX via an injected `gen`: the stub finds
    # which eval note is in the prompt and returns that record's gold as JSON. predict_with_model
    # builds the (shape-correct) prompt, calls gen, parses, and scores → a perfect self-equivalent
    # run. This pins the prompt-build → generate → parse → score wiring offline.
    import json as _json
    dictionary = _json.loads((ed.HERE.parent / "warm" / "dictionary.json").read_text())

    def stub_gen(prompt: str) -> str:
        rec = next(r for r in RECORDS if r["note"] in prompt)
        return "model says: " + _json.dumps(rec["gold"])  # prose wrapper exercises parse_ir too

    preds = ed.predict_with_model(RECORDS, stub_gen, dictionary)
    assert set(preds) == {r["id"] for r in RECORDS}
    _, agg = ed.score_predictions(RECORDS, preds)
    assert agg["fact_f1"] == 1.0 and agg["span_faithfulness"] == 1.0
    assert agg["near_miss_violations"] == 0 and agg["false_positive_facts"] == 0


def test_build_prompt_is_shape_appropriate_and_contains_the_note():
    cf = next(r for r in RECORDS if r["shape"] == "chart_facts")
    fnd = next(r for r in RECORDS if r["shape"] == "findings")
    import json as _json
    dictionary = _json.loads((ed.HERE.parent / "warm" / "dictionary.json").read_text())
    p_cf = ed.build_prompt(cf, dictionary)
    p_fnd = ed.build_prompt(fnd, dictionary)
    assert cf["note"] in p_cf and "chart" in p_cf.lower()
    assert fnd["note"] in p_fnd and "finding" in p_fnd.lower()


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
    print(f"\ntest_eval_decompose: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
