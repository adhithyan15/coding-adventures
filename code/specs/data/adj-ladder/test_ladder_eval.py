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
    "rung3_linear_systems",
    "rung3_constraint_feasibility",
    "rung3_probability_decisions",
    "rung3_derived_probability_decisions",
    "rung3_linear_optimization",
    "rung3_optimization_witness",
    "rung3_quadratic_roots",
    "rung3_cubic_roots",
    "rung3_quartic_roots",
    "rung3_factored_roots",
    "rung4_physics_chem",
    "rung4_dimensional",
    "rung4_products",
    "rung5_multistep",
    "rung6_clinical_differential",
    "rung6b_management",
    "rung6c_formulary_cost",
    "rung7_risk_measures",
    "rung7b_diagnostic_tests",
    "rung7c_likelihood_ratios",
    "rung8_pharmacokinetics",
    "rung9_fractional_excretion",
    "rung10_anion_gap",
    "rung11_syndromic_decision",
    "rung12_threshold_decision",
    "rung13_transtubular_gradient",
    "rung14_indeterminate_decision",
    "rung15_fractional_excretion_urea",
    "rung16_cardiac_output",
    "rung17_alveolar_ventilation",
    "rung18_ejection_fraction",
    "rung19_rbc_indices",
    "rung20_hepatic_indices",
    "rung21_renal_indices",
    "rung22_thyroid_indices",
    "rung23_iron_studies",
    "rung24_oxygen_extraction",
    "rung25_lipid_indices",
    "rung26_mineral_indices",
    "rung27_urine_anion_gap",
    "rung28_coagulation_ratios",
    "rung29_serum_protein_indices",
    "rung30_body_mass_index",
    "rung31_starling_filtration",
    "rung32_respiratory_exchange_ratio",
    "rung33_stroke_work",
    "rung34_fluid_admixture",
    "rung35_dialysis_clearance",
    "rung36_transfusion_pooling",
    "rung37_csf_serum_ratio",
    "rung38_serum_anion_gap",
    "rung39_wbc_differential",
    "rung40_lesion_volume",
    "rung41_split_renal_function",
    "rung42_hemofiltration_concentration",
    "rung43_compounded_admixture",
    "rung44_indicator_dilution",
    "rung45_elimination_rate",
    "rung46_cost_per_patient_day",
    "rung47_ingestion_concentration",
    "rung48_infusion_delivered_volume",
    "rung49_fractionated_dose",
    "rung50_insulin_correction",
    "rung51_dispensing_quantity",
    "rung52_alveolar_ventilation",
    "rung53_caloric_density",
    "rung54_neutrophil_ratio",
    "rung55_fresh_gas_volume",
    "rung56_stroke_work",
    "rung57_renal_count_rate",
    "rung58_pressor_concentration",
    "rung59_reconstituted_concentration",
    "rung60_apheresis_net_rate",
    "rung61_polysomnography_apnea_rate",
    "rung62_urodynamics_average_flow",
    "rung63_lacrimal_tear_clearance",
    "rung64_gastric_acid_buffer",
    "rung65_audiometry_corrected_threshold",
    "rung66_periodontal_attachment_level",
    "rung67_ergometry_specific_work",
    "rung68_gait_efficiency_index",
    "rung69_traction_force",
    "rung70_phototherapy_dose",
    "rung71_refraction_acuity",
    "rung72_amniotic_index",
    "rung73_spirometry_flow",
    "rung74_dialysis_clearance",
    "rung75_hearing_threshold",
    "rung76_fluid_resuscitation",
    "rung77_gas_uptake",
    "rung78_insulin_titration",
    "rung79_transfusion_load",
    "rung80_gi_motility",
    "rung81_cardiac_perfusion",
    "rung82_gfr_estimation",
    "rung83_hepatic_clearance",
    "rung84_intraocular_pressure",
    "rung85_wound_closure",
    "rung86_joint_fluid",
    "rung87_complement_titer",
    "rung88_nerve_conduction",
    "rung89_skin_test_wheal",
    "rung90_spirometry_reserve",
    "rung91_protein_delivery",
    "rung92_gi_absorption",
    "rung93_insulin_dosing",
    "rung94_toxin_clearance",
    "rung95_acoustic_reflex",
    "rung96_urodynamics",
    "rung97_amniotic_fluid",
    "rung98_segmental_pressure",
    "rung99_clearance_fraction",
    "rung100_refraction_focus",
    "rung101_range_of_motion",
    "rung102_training_load",
    "rung103_gi_transit",
    "rung104_coag_mixing",
    "rung105_sleep_study",
    "rung106_exercise_load",
    "rung107_contact_lens",
    "rung108_periodontal",
    "rung109_wound_care",
    "rung110_audiology",
    "rung111_dosimetry",
    "rung112_anesthesia",
    "rung113_apheresis",
    "rung114_fluency",
    "rung115_phlebotomy",
    "rung116_otolaryngology",
    "rung117_podiatry",
    "rung118_occupational_therapy",
    "rung119_allergy_skin_testing",
    "rung120_pulmonology_spirometry",
    "rung121_neonatology_perfusion",
    "rung122_rehabilitation_recovery",
    "rung123_sports_concussion_clearance",
    "rung124_vestibular_response_index",
    "rung125_electromyography_recruitment",
    "rung126_capnography_ventilation",
    "rung127_densitometry_index",
    "rung128_microperfusion_mean",
    "rung129_solute_clearance_mean",
    "rung130_relative_potency_ratio",
    "rung131_perfusate_throughput",
    "rung132_occupancy_share",
    "rung133_elimination_span",
    "rung134_cumulative_load_span",
    "rung135_admixture_concentration",
    "rung136_recovery_density",
    "rung137_gradient_index",
    "rung138_pressure_ratio",
    "rung139_slope_ratio",
    "rung140_yield_ratio",
    "rung141_grid_density",
    "rung142_packing_ratio",
    "rung143_shelf_density",
    "rung144_dose_density",
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


def test_probability_program_faithfulness_checks_priors_and_lrs_when_requested():
    program = "\n".join([
        "prior 0.30 for bacterial",
        "prior 0.30 for viral",
        "contributes 15 from csf(neutrophilic) to bacterial",
        "contributes 1.2 from csf(neutrophilic) to viral",
        "observe csf(neutrophilic)",
        "? bacterial",
        "? viral",
    ])
    stem = (
        "Two diagnoses start with prior 0.30 each. Evidence csf(neutrophilic) "
        "has likelihood ratio 15 for bacterial and 1.2 for viral."
    )
    assert le.formula_is_faithful(
        program, stem, program=True, structural_weights=False
    )
    leaked_stem = (
        "Two diagnoses start with prior 0.30 each. Evidence csf(neutrophilic) "
        "has likelihood ratio 1.2 for viral."
    )
    assert not le.formula_is_faithful(
        program, leaked_stem, program=True, structural_weights=False
    )


# ---- decision → letter --------------------------------------------------------
def test_determinate_maps_to_letter():
    assert le.decision_to_letter({"type": "determinate", "leader": "opt_c"}) == "C"


def test_kickback_abstains():
    assert le.decision_to_letter({"type": "kickback", "leader": "opt_a"}) is None


def test_missing_decision_abstains():
    assert le.decision_to_letter(None) is None


# ---- Rung-4 dimensional: (value, unit) option mapping -------------------------
_DIM_OPTIONS = {
    "A": {"value": 80, "unit": "km/h"},
    "B": {"value": 80, "unit": "m/s"},
    "C": {"value": 720, "unit": "km/h"},
    "D": {"value": 80, "unit": "km"},
    "E": {"value": 60, "unit": "km/h"},
}


def _dim_doc(value, unit, name="answer"):
    return {"derived": [{"name": name, "value": value, "dim": unit}]}


def test_dimensioned_match_requires_value_and_unit():
    af = {"type": "compute_dimensioned", "name": "answer"}
    # Right number AND right unit → the km/h option.
    assert le.compute_dimensioned_to_letter(_dim_doc(80, "km/h"), af, _DIM_OPTIONS) == "A"


def test_dimensioned_wrong_unit_is_not_matched():
    # The engine reports 80 km/h; an 80-m/s answer must NOT be read as the km/h
    # option — the unit discriminates, not just the magnitude.
    af = {"type": "compute_dimensioned", "name": "answer"}
    assert le.compute_dimensioned_to_letter(_dim_doc(80, "m/s"), af, _DIM_OPTIONS) == "B"


def test_dimensioned_unknown_unit_abstains():
    af = {"type": "compute_dimensioned", "name": "answer"}
    # A unit no option carries → no unique match → abstain (never a guess).
    assert le.compute_dimensioned_to_letter(_dim_doc(80, "furlong/fortnight"), af, _DIM_OPTIONS) is None


def test_dimensioned_missing_binding_abstains():
    af = {"type": "compute_dimensioned", "name": "answer"}
    assert le.compute_dimensioned_to_letter({"derived": []}, af, _DIM_OPTIONS) is None
    assert le.compute_dimensioned_to_letter({}, af, _DIM_OPTIONS) is None


def test_dimensioned_scalar_cancellation_maps():
    opts = {
        "A": {"value": 3, "unit": "mg"},
        "B": {"value": 3, "unit": "scalar"},
        "C": {"value": 3, "unit": "mg/mg"},
        "D": {"value": 1200, "unit": "scalar"},
        "E": {"value": 4, "unit": "scalar"},
    }
    af = {"type": "compute_dimensioned", "name": "answer"}
    assert le.compute_dimensioned_to_letter(_dim_doc(3, "scalar"), af, opts) == "B"


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


def test_decompose_prompt_mentions_linear_system_program():
    prompt = le.decompose_prompt({
        "stem": "Two numbers x and y have sum 10 and difference 2. What is x?",
        "program": (
            "symbol x : scalar\n"
            "symbol y : scalar\n"
            "constrain x + y = 10\n"
            "constrain x - y = 2\n"
            "solve for { x, y }\n"
        ),
        "answer_from": {"type": "solve_assignment", "name": "x"},
    })
    assert "symbol y : scalar" in prompt
    assert "solve for { x, y }" in prompt
    assert "include every variable in `solve for { ... }`" in prompt
    assert "the harness will read the requested variable" in prompt
    assert 'constrain latex "$x + 2y = 23$"' in prompt


def test_decompose_prompt_mentions_native_optimization_program():
    prompt = le.decompose_prompt({
        "stem": "Choose x and y with x and y at least 0. Maximize 3x + 2y.",
        "program": (
            "symbol x : scalar\n"
            "symbol y : scalar\n"
            "constrain x + y <= 4\n"
            "constrain x <= 3\n"
            "constrain x >= 0\n"
            "constrain y >= 0\n"
            "maximize 3 * x + 2 * y\n"
        ),
        "answer_from": {"type": "optimize_value"},
    })
    assert "linear optimization program" in prompt
    assert "maximize 3 * x + 2 * y" in prompt
    assert "minimize x + y" in prompt
    assert "Do NOT compute the optimum" in prompt


def test_decompose_prompt_mentions_native_check_program():
    prompt = le.decompose_prompt({
        "stem": "Is there a value of x with x >= 3 and x <= 5?",
        "program": "symbol x : scalar\nconstrain x >= 3\nconstrain x <= 5\ncheck\n",
        "answer_from": {"type": "check_outcome"},
    })
    assert "constraint feasibility program" in prompt
    assert "constrain x >= 3" in prompt
    assert "check" in prompt
    assert "Do NOT decide feasibility" in prompt


def test_decompose_prompt_mentions_native_probability_decision_program():
    prompt = le.decompose_prompt({
        "stem": (
            "Two diagnoses start with prior 0.30 each: bacterial and viral. "
            "Observed evidence is csf(neutrophilic). That evidence has likelihood "
            "ratio 15 for bacterial and 1.2 for viral. Which diagnosis leads?"
        ),
        "program": (
            "prior 0.30 for bacterial\n"
            "prior 0.30 for viral\n"
            "contributes 15 from csf(neutrophilic) to bacterial\n"
            "contributes 1.2 from csf(neutrophilic) to viral\n"
            "observe csf(neutrophilic)\n"
            "? bacterial\n"
            "? viral\n"
        ),
        "answer_from": {"type": "decision_leader", "structural_weights": False},
    })
    assert "probability decision program" in prompt
    assert "prior 0.30 for bacterial" in prompt
    assert "contributes 15 from csf(neutrophilic) to bacterial" in prompt
    assert "Do NOT choose the answer" in prompt


def test_decompose_prompt_mentions_derived_probability_requirements():
    prompt = le.decompose_prompt({
        "stem": (
            "Two diagnoses start with prior 0.05 for tuberculosis and 0.25 for "
            "bronchitis. Findings are prolonged_cough and night_sweats. Those "
            "findings derive tb_pattern. Tb_pattern has likelihood ratio 25 for "
            "tuberculosis and 0.5 for bronchitis. Which diagnosis leads?"
        ),
        "program": (
            "prior 0.05 for tuberculosis\n"
            "prior 0.25 for bronchitis\n"
            "contributes 25 from tb_pattern to tuberculosis\n"
            "contributes 0.5 from tb_pattern to bronchitis\n"
            "observe prolonged_cough\n"
            "observe night_sweats\n"
            "rule { head: tb_pattern when: prolonged_cough, night_sweats }\n"
            "? tuberculosis\n"
            "? bronchitis\n"
        ),
        "answer_from": {
            "type": "decision_leader",
            "structural_weights": False,
            "requires": [{
                "type": "decision",
                "leader": "tuberculosis",
                "evidence": "tb_pattern",
            }],
        },
    })
    assert "derived-evidence probability decision program" in prompt
    assert "rule { head: tb_pattern when: prolonged_cough, night_sweats }" in prompt
    assert "Required decision leader: tuberculosis" in prompt
    assert "Required derived evidence: tb_pattern" in prompt


def test_decompose_prompt_mentions_native_optimization_witness_program():
    prompt = le.decompose_prompt({
        "stem": "Choose x and y with x and y at least 0. Maximize 3x + 2y. What is x?",
        "program": (
            "symbol x : scalar\n"
            "symbol y : scalar\n"
            "constrain x + y <= 4\n"
            "constrain x <= 3\n"
            "constrain x >= 0\n"
            "constrain y >= 0\n"
            "maximize 3 * x + 2 * y\n"
        ),
        "answer_from": {"type": "optimize_assignment", "name": "x"},
    })
    assert "linear optimization program" in prompt
    assert "maximize 3 * x + 2 * y" in prompt
    assert "requested `x` witness value" in prompt
    assert "Do NOT compute" in prompt


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


def test_model_score_item_records_decomposition_trace():
    item = {
        "id": "trace_001",
        "stem": "What is 2 + 3?",
        "formula": "2 + 3",
        "options": {"A": 4, "B": 5, "C": 6, "D": 7, "E": 8},
        "gold_letter": "B",
    }
    result = le.score_item(item, lambda prompt: "Formula: 2 + 3")
    assert result.arm_b_model_output == "Formula: 2 + 3"
    assert result.arm_b_decomposition == "2 + 3"
    assert result.arm_b_decomposition_kind == "formula"
    assert result.arm_b_decomposition_faithful is True


def test_result_json_omits_trace_for_cached_result():
    result = le.ItemResult("1", "A", None, "abstained", "A", "correct", None)
    out = le.result_to_json(result)
    assert "arm_b_model_output" not in out
    assert out["arm_b"] == "A"


def test_result_json_includes_trace_for_model_result():
    result = le.ItemResult(
        "1",
        "A",
        "B",
        "wrong",
        None,
        "abstained",
        "b",
        "Formula: 99",
        "99",
        "formula",
        False,
        "solve",
        "no_unique_solution",
    )
    out = le.result_to_json(result)
    assert out["arm_b_model_output"] == "Formula: 99"
    assert out["arm_b_decomposition"] == "99"
    assert out["arm_b_decomposition_kind"] == "formula"
    assert out["arm_b_decomposition_faithful"] is False
    assert out["arm_b_engine_kind"] == "solve"
    assert out["arm_b_engine_outcome"] == "no_unique_solution"


def test_run_limit_caps_item_count():
    card = le.run("rung0_arithmetic", gen=None, limit=3)
    assert len(card.results) == 3
    assert card.summary()["arm_b_model_plus_adj"]["total"] == 3


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


def test_program_engine_trace_records_native_outcomes():
    assert le.program_engine_trace(
        {"solve": {"outcome": "no_unique_solution"}},
        {"type": "solve_assignment", "name": "x"},
    ) == ("solve", "no_unique_solution")
    assert le.program_engine_trace(
        {"optimize": {"outcome": "optimal"}},
        {"type": "optimize_value"},
    ) == ("optimize", "optimal")
    assert le.program_engine_trace(
        {"check": {"outcome": "sat"}},
        {"type": "check_outcome"},
    ) == ("check", "sat")
    assert le.program_engine_trace(
        {"decision": {"type": "determinate", "leader": "a"}},
        {"type": "decision_leader"},
    ) == ("decision", "determinate")


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


def test_solve_assignment_program_maps_linear_system_value_to_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    program = (
        "symbol x : scalar\n"
        "symbol y : scalar\n"
        "constrain x + y = 10\n"
        "constrain x - y = 2\n"
        "solve for { x, y }\n"
    )
    doc = le.run_program(program)
    assert le.program_answer_to_letter(
        doc,
        {"type": "solve_assignment", "name": "x"},
        {"A": 4, "B": 5, "C": 6, "D": 7, "E": 8},
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


def test_optimize_program_maps_engine_value_to_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    program = (
        "symbol x : scalar\n"
        "symbol y : scalar\n"
        "constrain x + y <= 4\n"
        "constrain x <= 3\n"
        "constrain x >= 0\n"
        "constrain y >= 0\n"
        "maximize 3 * x + 2 * y\n"
    )
    doc = le.run_program(program)
    assert le.program_answer_to_letter(
        doc,
        {"type": "optimize_value"},
        {"A": 9, "B": 10, "C": 11, "D": 12, "E": 13},
    ) == "C"


def test_optimize_program_maps_engine_witness_to_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    program = (
        "symbol x : scalar\n"
        "symbol y : scalar\n"
        "constrain x + y <= 4\n"
        "constrain x <= 3\n"
        "constrain x >= 0\n"
        "constrain y >= 0\n"
        "maximize 3 * x + 2 * y\n"
    )
    doc = le.run_program(program)
    assert le.program_answer_to_letter(
        doc,
        {"type": "optimize_assignment", "name": "x"},
        {"A": 1, "B": 2, "C": 3, "D": 4, "E": 5},
    ) == "C"


def test_check_program_maps_engine_outcome_to_label_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    doc = le.run_program("symbol x : scalar\nconstrain x >= 3\nconstrain x <= 5\ncheck\n")
    assert le.program_answer_to_letter(
        doc,
        {"type": "check_outcome"},
        {
            "A": "feasible",
            "B": "infeasible",
            "C": "unbounded",
            "D": "optimal",
            "E": "unknown",
        },
    ) == "A"


def test_check_program_maps_unsat_to_label_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    doc = le.run_program("symbol x : scalar\nconstrain x >= 5\nconstrain x <= 3\ncheck\n")
    assert le.program_answer_to_letter(
        doc,
        {"type": "check_outcome"},
        {
            "A": "feasible",
            "B": "infeasible",
            "C": "unbounded",
            "D": "optimal",
            "E": "unknown",
        },
    ) == "B"


def test_probability_decision_program_maps_engine_leader_to_label_option():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    doc = le.run_program(
        "prior 0.30 for bacterial\n"
        "prior 0.30 for viral\n"
        "contributes 15 from csf(neutrophilic) to bacterial\n"
        "contributes 1.2 from csf(neutrophilic) to viral\n"
        "observe csf(neutrophilic)\n"
        "? bacterial\n"
        "? viral\n"
    )
    assert le.program_answer_to_letter(
        doc,
        {"type": "decision_leader"},
        {
            "A": "bacterial",
            "B": "viral",
            "C": "fungal",
            "D": "migraine",
            "E": "unknown",
        },
    ) == "A"


def test_tied_decision_maps_kickback_to_tie_label_option():
    """rung-14: two equally-supported hypotheses → the engine kicks back, and with a
    `tie_label` the harness surfaces that abstention as the 'insufficient information' option."""
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    program = (
        "prior 0.2 for alpha\n"
        "prior 0.2 for beta\n"
        "prior 0.2 for gamma\n"
        "prior 0.2 for delta\n"
        "contributes 5 from clue_a to alpha\n"
        "contributes 5 from clue_b to beta\n"
        "observe clue_a\n"
        "observe clue_b\n"
        "? alpha\n? beta\n? gamma\n? delta\n"
    )
    doc = le.run_program(program)
    # The engine must report a genuine tie (kickback), not a determinate leader.
    assert isinstance(doc, dict) and doc.get("decision", {}).get("type") == "kickback"
    options = {
        "A": "alpha",
        "B": "beta",
        "C": "gamma",
        "D": "delta",
        "E": "insufficient information to distinguish",
    }
    # With a tie_label the kickback resolves to the abstention option E …
    assert le.program_answer_to_letter(
        doc,
        {"type": "decision_leader", "tie_label": "insufficient information to distinguish"},
        options,
    ) == "E"
    # … and WITHOUT a tie_label the same kickback stays an abstention (None), preserving
    # every existing decision rung's behaviour.
    assert le.program_answer_to_letter(doc, {"type": "decision_leader"}, options) is None


def test_derived_probability_decision_requires_evidence_proof():
    if le._CLI is None:
        pytest.skip("adj-lang-cli not built")
    program = (
        "prior 0.05 for tuberculosis\n"
        "prior 0.25 for bronchitis\n"
        "contributes 25 from tb_pattern to tuberculosis\n"
        "contributes 0.5 from tb_pattern to bronchitis\n"
        "observe prolonged_cough\n"
        "observe night_sweats\n"
        "rule { head: tb_pattern when: prolonged_cough, night_sweats }\n"
        "? tuberculosis\n"
        "? bronchitis\n"
    )
    answer_from = {
        "type": "decision_leader",
        "requires": [{
            "type": "decision",
            "leader": "tuberculosis",
            "evidence": "tb_pattern",
        }],
    }
    doc = le.run_program(program)
    assert le.program_requirements_hold(doc, answer_from)
    assert le.program_answer_to_letter(
        doc,
        answer_from,
        {
            "A": "bronchitis",
            "B": "tuberculosis",
            "C": "asthma",
            "D": "pneumonia",
            "E": "unknown",
        },
    ) == "B"


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


def test_contamination_check_accepts_check_outcome_labels(tmp_path, monkeypatch):
    rung = tmp_path / "check_labels"
    rung.mkdir()
    (rung / "items.json").write_text(json.dumps({"items": [{
        "id": "check_label_001",
        "qtype": "constraint_feasibility",
        "stem": "Is there a value of x with x >= 3 and x <= 5?",
        "program": "symbol x : scalar\nconstrain x >= 3\nconstrain x <= 5\ncheck\n",
        "answer_from": {"type": "check_outcome"},
        "options": {
            "A": "feasible",
            "B": "infeasible",
            "C": "unbounded",
            "D": "optimal",
            "E": "unknown",
        },
        "gold_letter": "A",
    }]}) + "\n")
    monkeypatch.setattr(cc, "HERE", tmp_path)
    assert cc.check("check_labels") == []


def test_contamination_check_accepts_decision_leader_labels(tmp_path, monkeypatch):
    rung = tmp_path / "decision_labels"
    rung.mkdir()
    (rung / "items.json").write_text(json.dumps({"items": [{
        "id": "decision_label_001",
        "qtype": "probability_decision",
        "stem": (
            "Two diagnoses start with prior 0.30 each: bacterial and viral. "
            "Evidence csf(neutrophilic) has likelihood ratio 15 for bacterial "
            "and 1.2 for viral."
        ),
        "program": (
            "prior 0.30 for bacterial\n"
            "prior 0.30 for viral\n"
            "contributes 15 from csf(neutrophilic) to bacterial\n"
            "contributes 1.2 from csf(neutrophilic) to viral\n"
            "observe csf(neutrophilic)\n"
            "? bacterial\n"
            "? viral\n"
        ),
        "answer_from": {"type": "decision_leader", "structural_weights": False},
        "options": {
            "A": "bacterial",
            "B": "viral",
            "C": "fungal",
            "D": "migraine",
            "E": "unknown",
        },
        "gold_letter": "A",
    }]}) + "\n")
    monkeypatch.setattr(cc, "HERE", tmp_path)
    assert cc.check("decision_labels") == []


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
