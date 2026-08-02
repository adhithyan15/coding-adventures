from __future__ import annotations

import copy
import math
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_backward_optimizer_lowering_labs import (
    DEFAULT_FIXTURE_ROOT,
    BackwardOptimizerLoweringValidationError,
    _compare,
    compile_backward_ir,
    compile_matrix_training_ir,
    compile_optimizer_ir,
    execute_scenario,
    load_json,
    validate_document,
    validate_fixture_root,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-scalar-sgd-training.json"


def load_lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_checked_in_corpus_and_schema_validate() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(load_lab())
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_exact_backward_optimizer_and_matrix_streams_are_pinned() -> None:
    document = validate_document(load_lab())
    backward = compile_backward_ir()
    optimizer = compile_optimizer_ir()
    matrix = compile_matrix_training_ir()

    assert [item["id"] for item in backward["instructions"]] == [
        f"b{index}" for index in range(6)
    ]
    assert [item["op"] for item in backward["instructions"]] == [
        "SEED_LOSS_GRAD",
        "HALF_SQUARED_ERROR_GRAD",
        "PROPAGATE_GRAD",
        "PARAMETER_LOCAL_GRAD",
        "ACCUMULATE_GRAD",
        "INPUT_GRAD",
    ]
    assert [item["op"] for item in optimizer["instructions"]] == [
        "READ_GRAD_BUFFER",
        "DIVIDE_GRAD",
        "SGD_UPDATE",
        "KEEP_GRAD_BUFFER",
    ]
    assert [item["op"] for item in matrix["instructions"]] == [
        "LOAD_SAVED_COLUMN",
        "LOAD_SAVED_COLUMN",
        "LOSS_GRAD_COLUMN",
        "PARAMETER_LOCAL_GRAD_COLUMN",
        "INPUT_GRAD_COLUMN",
        "REDUCE_SUM_GRAD",
        "ACCUMULATE_GRAD_BUFFER",
        "DIVIDE_GRAD",
        "SGD_UPDATE_SCALAR",
        "KEEP_GRAD_BUFFER",
    ]
    assert document["expected_ir"] == {
        "backward": backward,
        "optimizer": optimizer,
        "matrix_training": matrix,
    }


def test_one_row_replays_the_hand_calculation() -> None:
    document = validate_document(load_lab())
    scenario = document["scenarios"][0]
    trace = execute_scenario(scenario, document["finite_difference_epsilon"])

    assert trace["saved_values"] == {
        "x": [2],
        "target": [0],
        "prediction": [1],
        "residual": [1],
        "loss": [0.5],
    }
    assert trace["backward"] == {
        "d_loss": [1],
        "d_residual": [1],
        "d_prediction": [1],
        "local_d_w": [2],
        "d_x": [0.5],
        "gradient_buffer_before": 0,
        "batch_gradient": 2,
        "grad_w": 2,
    }
    assert trace["optimizer"]["parameter_after"] == pytest.approx(0.3)
    assert trace["optimizer"]["gradient_buffer_after_step"] == 2


def test_two_rows_reduce_stably_then_apply_the_explicit_mean() -> None:
    document = validate_document(load_lab())
    scenario = document["scenarios"][1]
    trace = execute_scenario(scenario, document["finite_difference_epsilon"])

    assert trace["saved_values"]["prediction"] == [2, -1]
    assert trace["backward"]["d_prediction"] == [1, -2]
    assert trace["backward"]["local_d_w"] == [2, 2]
    assert trace["backward"]["grad_w"] == 4
    assert trace["optimizer"]["applied_gradient"] == 2
    assert trace["optimizer"]["parameter_after"] == pytest.approx(0.8)
    assert trace["matrix_training"]["columns"]["d_x"] == [1, -2]


def test_nonzero_gradient_buffer_is_accumulated_and_kept() -> None:
    document = validate_document(load_lab())
    scenario = document["scenarios"][2]
    trace = execute_scenario(scenario, document["finite_difference_epsilon"])

    assert trace["backward"]["gradient_buffer_before"] == 3
    assert trace["backward"]["batch_gradient"] == 2
    assert trace["backward"]["grad_w"] == 5
    assert trace["matrix_training"]["batch_gradient"] == 2
    assert trace["matrix_training"]["grad_w"] == 5
    assert trace["optimizer"]["parameter_after"] == 0
    assert trace["optimizer"]["gradient_buffer_after_step"] == 5
    assert trace["gradient_audit"]["analytical"] == 2
    assert trace["max_path_error"] == 0


def test_every_scenario_passes_an_independent_gradient_audit() -> None:
    document = validate_document(load_lab())
    for scenario in document["scenarios"]:
        trace = execute_scenario(scenario, document["finite_difference_epsilon"])
        audit = trace["gradient_audit"]
        assert audit["numerical"] == pytest.approx(audit["analytical"], abs=1e-8)
        assert audit["absolute_error"] <= document["absolute_tolerance"]
        assert trace["max_path_error"] == 0


def test_rejects_duplicate_keys_non_finite_numbers_and_unknown_fields(
    tmp_path: Path,
) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(
        BackwardOptimizerLoweringValidationError, match="duplicate JSON key"
    ):
        load_json(duplicate)

    non_finite = tmp_path / "non-finite.json"
    non_finite.write_text('{"value": Infinity}', encoding="utf-8")
    with pytest.raises(BackwardOptimizerLoweringValidationError, match="non-finite"):
        load_json(non_finite)

    extra = load_lab()
    extra["surprise"] = True
    with pytest.raises(BackwardOptimizerLoweringValidationError, match="key mismatch"):
        validate_document(extra)


def test_rejects_noncanonical_contract_values() -> None:
    loose = load_lab()
    loose["absolute_tolerance"] = 1
    with pytest.raises(BackwardOptimizerLoweringValidationError, match="canonical"):
        validate_document(loose)

    wrong_epsilon = load_lab()
    wrong_epsilon["finite_difference_epsilon"] = 0.1
    with pytest.raises(BackwardOptimizerLoweringValidationError, match="canonical"):
        validate_document(wrong_epsilon)

    implicit_zero = load_lab()
    implicit_zero["training_graph"]["optimizer_step_zeroes_gradient"] = True
    with pytest.raises(
        BackwardOptimizerLoweringValidationError,
        match="canonical scalar training graph",
    ):
        validate_document(implicit_zero)


def test_rejects_mismatched_batches_divisors_large_values_and_roster_changes() -> None:
    mismatch = load_lab()
    mismatch["scenarios"][1]["targets"] = [1]
    with pytest.raises(BackwardOptimizerLoweringValidationError, match="same length"):
        validate_document(mismatch)

    divisor = load_lab()
    divisor["scenarios"][0]["divisor"] = 2
    with pytest.raises(BackwardOptimizerLoweringValidationError, match="batch length"):
        validate_document(divisor)

    huge = load_lab()
    huge["scenarios"][0]["inputs"] = [int("9" * 1000)]
    with pytest.raises(
        BackwardOptimizerLoweringValidationError, match="finite bounded"
    ):
        validate_document(huge)

    reordered = load_lab()
    reordered["scenarios"] = list(reversed(reordered["scenarios"]))
    with pytest.raises(BackwardOptimizerLoweringValidationError, match="ids expected"):
        validate_document(reordered)


def test_rejects_dishonest_ir_and_expected_trace() -> None:
    dishonest_ir = load_lab()
    dishonest_ir["expected_ir"]["backward"]["instructions"][3]["op"] = "MAGIC"
    with pytest.raises(
        BackwardOptimizerLoweringValidationError, match="expected 'MAGIC'"
    ):
        validate_document(dishonest_ir)

    dishonest_trace = load_lab()
    dishonest_trace["scenarios"][0]["expected"]["optimizer"]["parameter_after"] = 9
    with pytest.raises(BackwardOptimizerLoweringValidationError, match="expected 9"):
        validate_document(dishonest_trace)


def test_comparison_rejects_adversarial_nesting() -> None:
    deeply_nested: object = 0
    for _ in range(66):
        deeply_nested = [deeply_nested]
    with pytest.raises(
        BackwardOptimizerLoweringValidationError, match="nesting exceeds"
    ):
        _compare(deeply_nested, deeply_nested, 0.0, "hostile")


def test_all_derived_values_are_finite() -> None:
    document = validate_document(load_lab())
    for scenario in document["scenarios"]:
        trace = execute_scenario(scenario, document["finite_difference_epsilon"])
        stack: list[object] = [trace]
        while stack:
            value = stack.pop()
            if isinstance(value, dict):
                stack.extend(value.values())
            elif isinstance(value, list):
                stack.extend(value)
            elif isinstance(value, (int, float)) and not isinstance(value, bool):
                assert math.isfinite(value)


def test_validated_document_is_a_fresh_normalized_copy() -> None:
    source = load_lab()
    snapshot = copy.deepcopy(source)
    validated = validate_document(source)
    source["scenarios"][0]["inputs"][0] = 999

    assert validated["scenarios"][0]["inputs"] == snapshot["scenarios"][0]["inputs"]
