from __future__ import annotations

import math
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_tensor_broadcasting_labs import (  # noqa: E402
    DEFAULT_FIXTURE_ROOT,
    TensorBroadcastingValidationError,
    execute_case,
    load_json,
    validate_document,
    validate_fixture_root,
)


LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-shape-and-broadcasting.json"


def load_lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_checked_in_corpus_validates() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(load_lab())
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_outer_grid_reuses_both_inputs_and_sums_reverse_routes() -> None:
    document = validate_document(load_lab())
    trace = execute_case(document["cases"][0], 1e-5)

    assert trace["compatible"] is True
    assert trace["output_shape"] == [2, 3]
    assert trace["output_values"] == [11, 21, 31, 12, 22, 32]
    assert trace["left_expanded_axes"] == [1]
    assert trace["right_expanded_axes"] == [0]
    assert trace["left_gradient"] == [6, 15]
    assert trace["right_gradient"] == [5, 7, 9]
    assert trace["max_gradient_absolute_error"] < 1e-8

    selected = trace["mappings"][4]
    assert selected["output_index"] == [1, 1]
    assert selected["left_index"] == [1, 0]
    assert selected["right_index"] == [0, 1]
    assert selected["left_value"] + selected["right_value"] == 22


def test_rank_padding_scalar_expansion_and_mismatch_are_explicit() -> None:
    document = validate_document(load_lab())
    cases = document["cases"]

    row = execute_case(cases[1], 1e-5)
    assert row["padded_right_shape"] == [1, 3]
    assert row["right_gradient"] == [2, 2, 2]

    scalar = execute_case(cases[2], 1e-5)
    assert scalar["padded_left_shape"] == [1, 1]
    assert scalar["left_expanded_axes"] == [0, 1]
    assert scalar["left_gradient"] == [0]

    mismatch = execute_case(cases[3], 1e-5)
    assert mismatch == {
        "compatible": False,
        "padded_left_shape": [2, 3],
        "padded_right_shape": [1, 2],
        "mismatch_axis": 1,
        "left_dimension": 3,
        "right_dimension": 2,
        "error": "axis 1: dimensions 3 and 2 are incompatible",
    }


def test_rejects_duplicate_keys_and_non_finite_numbers(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(TensorBroadcastingValidationError, match="duplicate JSON key"):
        load_json(duplicate)

    non_finite = tmp_path / "non-finite.json"
    non_finite.write_text('{"value": NaN}', encoding="utf-8")
    with pytest.raises(TensorBroadcastingValidationError, match="non-finite"):
        load_json(non_finite)


def test_rejects_unknown_fields_and_bad_case_roster() -> None:
    extra = load_lab()
    extra["surprise"] = True
    with pytest.raises(TensorBroadcastingValidationError, match="key mismatch"):
        validate_document(extra)

    reordered = load_lab()
    reordered["cases"] = list(reversed(reordered["cases"]))
    with pytest.raises(TensorBroadcastingValidationError, match="case ids"):
        validate_document(reordered)

    loose = load_lab()
    loose["absolute_tolerance"] = 1e308
    loose["cases"][0]["expected"]["output_values"][4] = 999
    with pytest.raises(TensorBroadcastingValidationError, match="canonical"):
        validate_document(loose)


def test_rejects_invalid_tensor_shapes_values_and_upstream() -> None:
    bad_dimension = load_lab()
    bad_dimension["cases"][0]["left"]["shape"] = [2, 0]
    with pytest.raises(TensorBroadcastingValidationError, match="positive integer"):
        validate_document(bad_dimension)

    bad_buffer = load_lab()
    bad_buffer["cases"][0]["left"]["values"] = [1]
    with pytest.raises(TensorBroadcastingValidationError, match="value count"):
        validate_document(bad_buffer)

    bad_upstream = load_lab()
    bad_upstream["cases"][0]["upstream"]["shape"] = [3, 2]
    with pytest.raises(TensorBroadcastingValidationError, match="upstream shape"):
        validate_document(bad_upstream)

    huge_number = load_lab()
    huge_number["cases"][0]["left"]["values"][0] = int("9" * 1000)
    with pytest.raises(TensorBroadcastingValidationError, match="finite number"):
        validate_document(huge_number)

    huge_expected_number = load_lab()
    huge_expected_number["cases"][0]["expected"]["output_values"][0] = int("9" * 1000)
    with pytest.raises(TensorBroadcastingValidationError, match="finite number"):
        validate_document(huge_expected_number)


def test_rejects_dishonest_expected_trace() -> None:
    document = load_lab()
    document["cases"][0]["expected"]["output_values"][4] = 999
    with pytest.raises(TensorBroadcastingValidationError, match="expected 999"):
        validate_document(document)


def test_finite_difference_outputs_are_finite() -> None:
    document = validate_document(load_lab())
    for case in document["cases"][:3]:
        trace = execute_case(case, 1e-5)
        numbers = (
            trace["finite_difference_left_gradient"]
            + trace["finite_difference_right_gradient"]
        )
        assert all(math.isfinite(value) for value in numbers)
