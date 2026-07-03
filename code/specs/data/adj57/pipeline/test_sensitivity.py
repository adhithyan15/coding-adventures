#!/usr/bin/env python3
"""Tests for the sensitivity engine (sensitivity.py). Run: python test_sensitivity.py"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sensitivity as s  # noqa: E402

HYPS = ["brucellosis", "tuberculosis", "malaria"]


def ev(name, weights, source="grounded", citation="x"):
    return {"name": name, "weights": weights, "source": source, "citation": citation}


def test_decision_is_argmax_of_summed_decibans():
    e = [ev("granuloma", {"brucellosis": 5, "tuberculosis": 8, "malaria": -10}),
         ev("travel", {"brucellosis": 4, "tuberculosis": 1, "malaria": 6})]
    r = s.assess(HYPS, e)
    # tb: 9, bruc: 9, mal: -4 -> tie at top; argmax picks the first max (brucellosis or tb)
    assert r["scores"] == {"brucellosis": 9.0, "tuberculosis": 9.0, "malaria": -4.0}
    assert r["decision"] in ("brucellosis", "tuberculosis")


def test_margin_and_odds():
    e = [ev("a", {"brucellosis": 10, "tuberculosis": 0, "malaria": 0})]
    r = s.assess(HYPS, e)
    assert r["decision"] == "brucellosis"
    assert r["margin_db"] == 10.0 and r["margin_odds"] == 10.0  # +10 dB = 10x odds over runner-up


def test_posteriors_are_a_view_and_sum_to_one():
    raw = s.posteriors({"brucellosis": 10.0, "tuberculosis": 0.0, "malaria": 0.0})
    assert abs(sum(raw.values()) - 1.0) < 1e-12          # the function is exact
    r = s.assess(HYPS, [ev("a", {"brucellosis": 10, "tuberculosis": 0, "malaria": 0})])
    assert abs(sum(r["posteriors"].values()) - 1.0) < 1e-3  # the report rounds for display
    assert r["posteriors"]["brucellosis"] > r["posteriors"]["tuberculosis"]


def test_load_bearing_and_decisive_alone():
    e = [ev("big", {"brucellosis": 20, "tuberculosis": 0, "malaria": 0}),
         ev("small", {"brucellosis": 2, "tuberculosis": 0, "malaria": 0})]
    r = s.assess(HYPS, e)
    # leader bruc=22, runner tb=0, margin=22. 'big' pushes 20 (< 22, not decisive alone here)
    top = r["load_bearing"][0]
    assert top["name"] == "big" and top["push_for_leader"] == 20.0


def test_one_out_flip_detected():
    e = [ev("decisive", {"brucellosis": 30, "tuberculosis": 0, "malaria": 0}),
         ev("tb_lean", {"brucellosis": 0, "tuberculosis": 10, "malaria": 0})]
    r = s.assess(HYPS, e)
    assert r["decision"] == "brucellosis"
    # removing 'decisive' -> tb wins
    assert any(f["remove"] == "decisive" and f["new_leader"] == "tuberculosis" for f in r["one_out_flips"])


def test_min_facts_to_flip():
    e = [ev("a", {"brucellosis": 6, "tuberculosis": 0, "malaria": 0}),
         ev("b", {"brucellosis": 6, "tuberculosis": 0, "malaria": 0}),
         ev("c", {"brucellosis": 6, "tuberculosis": 0, "malaria": 0})]
    r = s.assess(HYPS, e)  # bruc=18, margin=18; need to erode >18 -> all 3 (6+6+6=18 not >18)... 4th not exist
    # eroding a(6)+b(6)+c(6)=18, not >18, so k stays None (cannot flip by removing supportive facts alone)
    assert r["min_facts_to_flip"] is None


def test_margin_rests_on_assumed_flagged():
    e = [ev("ungrounded_big", {"brucellosis": 20, "tuberculosis": 0, "malaria": 0}, source="assumed"),
         ev("grounded_small", {"brucellosis": 3, "tuberculosis": 0, "malaria": 0}, source="grounded")]
    r = s.assess(HYPS, e)
    assert r["margin_rests_on_assumed"] is True
    assert "ungrounded_big" in r["assumed_load_bearing"]


def test_tip_reports_threshold_and_target():
    e = [ev("a", {"brucellosis": 30, "tuberculosis": 0, "malaria": 0}),
         ev("b", {"brucellosis": 0, "tuberculosis": 10, "malaria": 0})]
    t = s.tip(HYPS, e, {}, "a")
    # bruc=30, tb=10, margin=20; weight 'a' for leader=30 > 20 -> can flip to tb
    assert t["can_flip_alone"] is True and t["flips_to"] == "tuberculosis"
    assert t["flip_needs_drop_db"] == 20.0


def test_tip_cannot_flip_when_weight_below_margin():
    e = [ev("a", {"brucellosis": 30, "tuberculosis": 0, "malaria": 0}),
         ev("small", {"brucellosis": 5, "tuberculosis": 0, "malaria": 0})]
    t = s.tip(HYPS, e, {}, "small")  # margin 35; small weight 5 < 35
    assert t["can_flip_alone"] is False and t["flips_to"] is None


def test_prior_shifts_the_decision():
    e = [ev("a", {"brucellosis": 5, "tuberculosis": 0, "malaria": 0})]
    r0 = s.assess(HYPS, e)
    r1 = s.assess(HYPS, e, prior={"tuberculosis": 10})  # strong prior for tb overrides +5 evidence
    assert r0["decision"] == "brucellosis" and r1["decision"] == "tuberculosis"


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in fns:
        fn()
        print(f"  ok  {fn.__name__}")
    print(f"\n{len(fns)} tests passed.")
