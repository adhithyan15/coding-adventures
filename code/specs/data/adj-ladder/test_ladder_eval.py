#!/usr/bin/env python3
"""Tests for the ADJ-LADDER harness.

These cover the parts that have NO model and NO engine dependency (program building,
faithfulness gate, decision→letter mapping, formula extraction, scoring/divergence
math) plus, when the adj-lang-cli binary is present, end-to-end cached rung runs
asserting the engine selects every gold option exactly. The bank-integrity checks live
here too.

Run:  python3 -m pytest test_ladder_eval.py -q
"""

from __future__ import annotations

import json
from pathlib import Path

import contamination_check as cc
import ladder_eval as le
import pytest

HERE = Path(__file__).resolve().parent
SELF_CONTAINED_RUNGS = (
    "rung0_arithmetic",
    "rung1_fractions_percent",
    "rung2_prealgebra_solve",
    "rung2_derived_solve",
    "rung3_quadratic_roots",
    "rung3_cubic_roots",
    "rung3_quartic_roots",
    "rung3_factored_roots",
)


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


def test_build_program_accepts_option_expressions():
    prog = le.build_arm_b_program("1 / 10 + 2 / 10", {"A": "3 / 10", "B": "1 / 2"})
    assert "let answer = 1 / 10 + 2 / 10" in prog
    assert "contributes 1000000 from answer == 3 / 10 to opt_a" in prog
    assert "contributes 1000000 from answer == 1 / 2 to opt_b" in prog


# ---- faithfulness / no-result-literals ----------------------------------------
def test_faithful_formula_passes():
    assert le.formula_is_faithful("7 * 8 + 3", "What is 7 * 8 + 3?")


def test_result_literal_is_rejected():
    # 59 is the ANSWER, not in the stem → must be rejected.
    assert not le.formula_is_faithful("59", "What is 7 * 8 + 3?")


def test_extra_number_rejected():
    assert not le.formula_is_faithful("7 * 8 + 5", "What is 7 * 8 + 3?")


def test_program_faithfulness_ignores_structural_weights_only():
    program = "\n".join([
        "prior 0.001 for setup_ready",
        "contributes 1000000 from repeated_groups_with_extra to setup_ready",
        "observe groups(9)",
        "observe per_group(6)",
        "observe extra(3)",
        "constrain x = groups * per_group + extra",
    ])
    stem = "There are 9 rows with 6 chairs in each row, and 3 chairs are added."
    assert le.formula_is_faithful(program, stem, program=True)
    assert not le.formula_is_faithful(program + "\nconstrain x = 57", stem, program=True)


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


def test_decompose_prompt_mentions_native_solve_program():
    prompt = le.decompose_prompt({
        "stem": "A number plus 7 equals 19. What is the number?",
        "program": "symbol x : scalar\nconstrain x + 7 = 19\nsolve for { x }\n",
        "answer_from": {"type": "solve_assignment", "name": "x"},
    })
    assert "symbol x : scalar" in prompt
    assert "solve for { x }" in prompt
    assert "Do NOT compute the answer" in prompt


def test_decompose_prompt_mentions_derived_solve_requirements():
    prompt = le.decompose_prompt({
        "stem": "There are 7 boxes with 8 pencils in each box. How many pencils are there?",
        "program": (
            "prior 0.001 for setup_ready\n"
            "contributes 1000000 from repeated_groups_problem to setup_ready\n"
            "observe groups(7)\n"
            "observe per_group(8)\n"
            "rule { head: repeated_groups_problem when: groups(7), per_group(8) }\n"
            "? setup_ready\n"
            "symbol x : scalar\n"
            "constrain x = groups * per_group\n"
            "solve for { x }\n"
        ),
        "answer_from": {
            "type": "solve_assignment",
            "name": "x",
            "requires": [{
                "type": "decision",
                "leader": "setup_ready",
                "evidence": "repeated_groups_problem",
            }],
        },
    })
    assert "derive the setup premise" in prompt
    assert "Required decision leader: setup_ready" in prompt
    assert "Required derived evidence: repeated_groups_problem" in prompt


def test_decompose_prompt_mentions_native_root_solve_program():
    prompt = le.decompose_prompt({
        "stem": "What real values of x solve x^2 = 4?",
        "program": "symbol x : scalar\nconstrain x * x = 4\nsolve for { x }\n",
        "answer_from": {"type": "solve_roots", "name": "x"},
    })
    assert "finds all real roots" in prompt
    assert "constrain x * x = 121" in prompt
    assert 'constrain latex "$x^2 = 144$"' in prompt
    assert "x * x * x - 6 * x * x + 11 * x - 6 = 0" in prompt
    assert "x * x * x * x - 10 * x * x * x + 35 * x * x - 50 * x + 24 = 0" in prompt
    assert "(x - 2) * (x - 5) = 0" in prompt


def test_extract_formula_abstains_on_latex():
    # Bare LaTeX/unicode math is NOT normalized in the harness. The model must
    # emit native ADJ syntax (`latex "..."`) so adj-lang owns parsing.
    assert le.extract_formula("$5 \\times 12$") is None
    assert le.extract_formula("5 × 12") is None


def test_extract_program_accepts_native_adj_solve_block():
    text = """Here is the program:
```adj
symbol x : scalar
constrain x + 7 = 19
solve for { x }
```
"""
    assert le.extract_program(text) == "symbol x : scalar\nconstrain x + 7 = 19\nsolve for { x }\n"


def test_extract_program_accepts_derived_solve_block():
    text = """```adj
prior 0.001 for setup_ready
contributes 1000000 from repeated_groups_problem to setup_ready
observe groups(7)
observe per_group(8)
rule { head: repeated_groups_problem when: groups(7), per_group(8) }
? setup_ready
symbol x : scalar
constrain x = groups * per_group
solve for { x }
```"""
    program = le.extract_program(text)
    assert program is not None
    assert "rule { head: repeated_groups_problem" in program
    assert "? setup_ready" in program


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
@pytest.mark.parametrize("rung", SELF_CONTAINED_RUNGS)
def test_cached_engine_selects_every_gold(rung):
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    card = le.run(rung, gen=None)
    s = card.summary()["arm_b_model_plus_adj"]
    # The engine must compute every arithmetic answer exactly and select the gold
    # option — zero wrong, zero abstain.
    assert s["wrong"] == 0, [r.item_id for r in card.results if r.arm_b_outcome == "wrong"]
    assert s["correct"] == s["total"], [r.item_id for r in card.results if r.arm_b_outcome != "correct"]


def test_native_adj_latex_formula_runs_through_engine():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    decision = le.run_decision(
        le.build_arm_b_program(r'latex "$5 \times 12$"', {"A": 60.0, "B": 61.0})
    )
    assert le.decision_to_letter(decision) == "A"


def test_option_expression_predicate_runs_through_engine():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    decision = le.run_decision(
        le.build_arm_b_program("1 / 10 + 2 / 10", {"A": "3 / 10", "B": "1 / 2"})
    )
    assert le.decision_to_letter(decision) == "A"


def test_solve_assignment_program_maps_engine_value_to_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    doc = le.run_program("symbol x : scalar\nconstrain x + 7 = 19\nsolve for { x }\n")
    assert le.solve_assignment_to_letter(
        doc,
        {"type": "solve_assignment", "name": "x"},
        {"A": 10, "B": 11, "C": 12, "D": 13, "E": 14},
    ) == "C"


def test_solve_assignment_requires_derived_decision_proof():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    program = (
        "prior 0.001 for setup_ready\n"
        "contributes 1000000 from repeated_groups_problem to setup_ready\n"
        "observe groups(7)\n"
        "observe per_group(8)\n"
        "rule { head: repeated_groups_problem when: groups(7), per_group(8) }\n"
        "? setup_ready\n"
        "symbol x : scalar\n"
        "constrain x = groups * per_group\n"
        "solve for { x }\n"
    )
    answer_from = {
        "type": "solve_assignment",
        "name": "x",
        "requires": [{
            "type": "decision",
            "leader": "setup_ready",
            "evidence": "repeated_groups_problem",
        }],
    }
    doc = le.run_program(program)
    assert le.program_requirements_hold(doc, answer_from)
    assert le.solve_assignment_to_letter(
        doc,
        answer_from,
        {"A": 48, "B": 54, "C": 56, "D": 63, "E": 64},
    ) == "C"


def test_solve_roots_program_maps_engine_roots_to_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    doc = le.run_program("symbol x : scalar\nconstrain x * x = 4\nsolve for { x }\n")
    assert le.solve_roots_to_letter(
        doc,
        {"type": "solve_roots", "name": "x"},
        {"A": [-3, 3], "B": [0, 2], "C": [-2, 2], "D": [2, 4], "E": [-4, 4]},
    ) == "C"


def test_solve_roots_program_maps_native_latex_roots_to_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    doc = le.run_program("symbol x : scalar\nconstrain latex \"$x^2 = 25$\"\nsolve for { x }\n")
    assert le.program_answer_to_letter(
        doc,
        {"type": "solve_roots", "name": "x"},
        {"A": [-6, 6], "B": [-25, 25], "C": [0, 5], "D": [5, 25], "E": [-5, 5]},
    ) == "E"


def test_solve_roots_program_maps_native_quartic_roots_to_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    program = (
        "symbol x : scalar\n"
        "constrain latex \"$x^4 - 10x^3 + 35x^2 - 50x + 24 = 0$\"\n"
        "solve for { x }\n"
    )
    doc = le.run_program(program)
    assert le.program_answer_to_letter(
        doc,
        {"type": "solve_roots", "name": "x"},
        {
            "A": [1, 2, 3, 5],
            "B": [0, 2, 3, 4],
            "C": [1, 2, 3, 4],
            "D": [1, 3, 4, 10],
            "E": [-1, 2, 3, 4],
        },
    ) == "C"


def test_solve_roots_program_maps_native_factored_latex_roots_to_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    program = (
        "symbol x : scalar\n"
        "constrain latex \"$(x + 2)(x - 3)(x - 6) = 0$\"\n"
        "solve for { x }\n"
    )
    doc = le.run_program(program)
    assert le.program_answer_to_letter(
        doc,
        {"type": "solve_roots", "name": "x"},
        {
            "A": [-2, 3, 6],
            "B": [2, 3, 6],
            "C": [-3, 2, 6],
            "D": [-2, 0, 6],
            "E": [-2, 3, 8],
        },
    ) == "A"


# ---- bank integrity -----------------------------------------------------------
@pytest.mark.parametrize("rung", SELF_CONTAINED_RUNGS)
def test_contamination_check_clean(rung):
    assert cc.check(rung) == []


def test_contamination_check_accepts_option_expressions(tmp_path, monkeypatch):
    rung = tmp_path / "expr_options"
    rung.mkdir()
    (rung / "items.json").write_text(json.dumps({"items": [{
        "id": "frac_expr_001",
        "qtype": "fraction",
        "stem": "What is 1/10 + 2/10?",
        "formula": "1 / 10 + 2 / 10",
        "options": {"A": "3 / 10", "B": "1 / 2", "C": "1 / 10", "D": "2 / 10", "E": "4 / 10"},
        "gold_letter": "A",
    }]}) + "\n")
    monkeypatch.setattr(cc, "HERE", tmp_path)
    assert cc.check("expr_options") == []


def test_safe_eval_rejects_code():
    import pytest
    with pytest.raises(ValueError):
        cc.safe_eval("__import__('os').system('echo hi')")


@pytest.mark.parametrize("rung", SELF_CONTAINED_RUNGS)
def test_items_json_valid(rung):
    data = json.loads((HERE / rung / "items.json").read_text())
    assert len(data["items"]) >= 20
    for it in data["items"]:
        assert set(it["options"]) == set("ABCDE")
