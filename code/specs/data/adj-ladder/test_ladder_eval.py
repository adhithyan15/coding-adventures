#!/usr/bin/env python3
"""Tests for the ADJ-LADDER harness.

These cover the parts that have NO model and NO engine dependency (program building,
faithfulness gate, decision→letter mapping, formula extraction, scoring/divergence
math) plus, when the adj-lang-cli binary is present, an end-to-end cached run of rung 0
asserting the engine selects every gold option exactly. The bank-integrity checks live
in test_contamination too.

Run:  python3 -m pytest test_ladder_eval.py -q
"""

from __future__ import annotations

import json
from pathlib import Path

import contamination_check as cc
import ladder_eval as le

HERE = Path(__file__).resolve().parent


# ---- Arm-B program building ---------------------------------------------------
def test_build_program_shape():
    prog = le.build_arm_b_program("7 * 8 + 3", {"A": 59, "B": 60})
    assert "let answer = 7 * 8 + 3" in prog
    assert "contributes 1000000 from answer == 59 to opt_a" in prog
    assert "contributes 1000000 from answer == 60 to opt_b" in prog
    assert "? opt_a" in prog and "? opt_b" in prog
    assert "prior 0.0001 for opt_a" in prog


def test_build_program_renders_int_thresholds():
    # whole-valued floats render without a trailing .0
    prog = le.build_arm_b_program("2 + 2", {"A": 4.0, "B": 5.0})
    assert "answer == 4 to opt_a" in prog
    assert "answer == 5 to opt_b" in prog


# ---- faithfulness / no-result-literals ----------------------------------------
def test_faithful_formula_passes():
    assert le.formula_is_faithful("7 * 8 + 3", "What is 7 * 8 + 3?")


def test_result_literal_is_rejected():
    # 59 is the ANSWER, not in the stem → must be rejected.
    assert not le.formula_is_faithful("59", "What is 7 * 8 + 3?")


def test_extra_number_rejected():
    assert not le.formula_is_faithful("7 * 8 + 5", "What is 7 * 8 + 3?")


# ---- decision → letter --------------------------------------------------------
def test_determinate_maps_to_letter():
    assert le.decision_to_letter({"type": "determinate", "leader": "opt_c"}) == "C"


def test_kickback_abstains():
    assert le.decision_to_letter({"type": "kickback", "leader": "opt_a"}) is None


def test_missing_decision_abstains():
    assert le.decision_to_letter(None) is None


# ---- Arm-A letter parsing -----------------------------------------------------
def test_parse_letter_variants():
    assert le.parse_letter("Answer: C") == "C"
    assert le.parse_letter("(b)") == "B"
    assert le.parse_letter("the answer is e.") == "E"
    assert le.parse_letter("I don't know") is None


# ---- formula extraction (model mode) ------------------------------------------
def test_extract_formula_picks_arithmetic_line():
    assert le.extract_formula("Sure! Here it is:\n7 * 8 + 3\nThat's it.") == "7 * 8 + 3"


def test_extract_formula_rejects_prose():
    assert le.extract_formula("fifty nine") is None


def test_extract_formula_strips_label_only():
    # a "Formula:" label the model echoes is stripped; plain arithmetic passes through.
    assert le.extract_formula("Formula: 5 * 12") == "5 * 12"


def test_extract_formula_accepts_native_adj_latex_expr():
    assert le.extract_formula(r'Formula: latex "$5 \times 12$"') == r'latex "$5 \times 12$"'


def test_decompose_prompt_mentions_native_adj_latex_expr():
    prompt = le.decompose_prompt({"stem": "What is 5 times 12?"})
    assert 'latex "$5 \\times 12$"' in prompt


def test_extract_formula_abstains_on_latex():
    # Bare LaTeX/unicode math is NOT normalized in the harness. The model must
    # emit native ADJ syntax (`latex "..."`) so adj-lang owns parsing.
    assert le.extract_formula("$5 \\times 12$") is None
    assert le.extract_formula("5 × 12") is None


def test_model_aliases_resolve():
    assert le.MODEL_ALIASES["gemma"].startswith("mlx:")
    assert "gemma" in le.MODEL_ALIASES["gemma"]
    assert le.MODEL_ALIASES["gemma-1b"].endswith("gemma-3-1b-it-bf16")


# ---- scoring / divergence -----------------------------------------------------
def test_outcome():
    assert le.outcome("A", "A") == "correct"
    assert le.outcome("B", "A") == "wrong"
    assert le.outcome(None, "A") == "abstained"


def test_bucket_classification():
    assert le.classify_bucket("correct", True) is None
    assert le.classify_bucket("wrong", False) == "b"     # unfaithful decompose
    assert le.classify_bucket("wrong", True) == "c"      # engine gap
    assert le.classify_bucket("abstained", True) == "c"


def test_divergence_math():
    card = le.Scorecard("r", "model")
    card.results = [
        le.ItemResult("1", "A", "A", "correct", "A", "correct", None),
        le.ItemResult("2", "B", "C", "wrong", "B", "correct", None),
        le.ItemResult("3", "C", None, "abstained", "C", "correct", None),
    ]
    s = card.summary()
    assert s["arm_a_model_alone"]["correct"] == 1
    assert s["arm_b_model_plus_adj"]["correct"] == 3
    assert s["divergence"]["correct"] == 2


# ---- end-to-end engine run (only when the CLI is built) -----------------------
def test_cached_engine_selects_every_gold():
    if le._CLI is None:
        import pytest
        pytest.skip("adj-lang-cli not built")
    card = le.run("rung0_arithmetic", gen=None)
    s = card.summary()["arm_b_model_plus_adj"]
    # The engine must compute every arithmetic answer exactly and select the gold
    # option — zero wrong, zero abstain.
    assert s["wrong"] == 0, [r.item_id for r in card.results if r.arm_b_outcome == "wrong"]
    assert s["correct"] == s["total"], [r.item_id for r in card.results if r.arm_b_outcome != "correct"]


def test_native_adj_latex_formula_runs_through_engine():
    if le._CLI is None:
        import pytest
        pytest.skip("adj-lang-cli not built")
    decision = le.run_decision(
        le.build_arm_b_program(r'latex "$5 \times 12$"', {"A": 60.0, "B": 61.0})
    )
    assert le.decision_to_letter(decision) == "A"


# ---- bank integrity -----------------------------------------------------------
def test_contamination_check_clean():
    assert cc.check("rung0_arithmetic") == []


def test_safe_eval_rejects_code():
    import pytest
    with pytest.raises(ValueError):
        cc.safe_eval("__import__('os').system('echo hi')")


def test_items_json_valid():
    data = json.loads((HERE / "rung0_arithmetic" / "items.json").read_text())
    assert len(data["items"]) >= 20
    for it in data["items"]:
        assert set(it["options"]) == set("ABCDE")
