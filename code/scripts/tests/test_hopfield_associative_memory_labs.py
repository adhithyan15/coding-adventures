from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPTS_DIR = REPO_ROOT / "code" / "scripts"
sys.path.insert(0, str(SCRIPTS_DIR))

from validate_hopfield_associative_memory_labs import (
    DEFAULT_FIXTURE_ROOT,
    HopfieldAssociativeMemoryValidationError,
    execute_lab,
    load_json,
    validate_corpus,
    validate_document,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-one-bit-recall.json"


def lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_schema_accepts_the_canonical_document() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(lab())


def test_corpus_contains_one_validated_lab() -> None:
    assert validate_corpus() == 1


def test_hebbian_outer_product_is_symmetric_with_zero_diagonal() -> None:
    result = execute_lab(lab())
    assert result["weights"] == [
        [0, -0.25, 0.25, -0.25],
        [-0.25, 0, -0.25, 0.25],
        [0.25, -0.25, 0, -0.25],
        [-0.25, 0.25, -0.25, 0],
    ]


def test_corrupted_cue_begins_at_half_overlap_and_zero_energy() -> None:
    result = execute_lab(lab())
    assert result["initial_hamming_distance"] == 1
    assert result["initial_overlap"] == pytest.approx(0.5)
    assert result["initial_energy"] == pytest.approx(0)


def test_first_asynchronous_update_repairs_the_flipped_bit() -> None:
    first = execute_lab(lab())["updates"][0]
    assert [row["contribution"] for row in first["incoming"]] == pytest.approx(
        [0, 0.25, 0.25, 0.25]
    )
    assert first["local_field"] == pytest.approx(0.75)
    assert first["previous_state"] == -1
    assert first["next_state"] == 1
    assert first["state_after"] == [1, -1, 1, -1]


def test_energy_descends_and_the_remaining_sweep_is_stable() -> None:
    result = validate_document(lab())
    assert [
        (row["energy_before"], row["energy_after"]) for row in result["updates"]
    ] == [
        (0, -1.5),
        (-1.5, -1.5),
        (-1.5, -1.5),
        (-1.5, -1.5),
    ]
    assert [row["changed"] for row in result["updates"]] == [True, False, False, False]
    assert result["final_state"] == [1, -1, 1, -1]
    assert result["final_overlap"] == pytest.approx(1)
    assert result["final_hamming_distance"] == 0
    assert result["converged"] is True


def test_rejects_unknown_document_keys() -> None:
    document = lab()
    document["surprise"] = True
    with pytest.raises(HopfieldAssociativeMemoryValidationError, match="key mismatch"):
        validate_document(document)


def test_rejects_non_bipolar_states() -> None:
    document = lab()
    document["corrupted_state"][0] = 0
    with pytest.raises(HopfieldAssociativeMemoryValidationError, match="bipolar"):
        validate_document(document)


def test_rejects_non_permutation_update_orders() -> None:
    document = lab()
    document["update_order"] = [0, 1, 1, 3]
    with pytest.raises(HopfieldAssociativeMemoryValidationError, match="permutation"):
        validate_document(document)


def test_rejects_a_mismatched_expected_trace() -> None:
    document = lab()
    document["expected"]["final_energy"] = -1.4
    with pytest.raises(HopfieldAssociativeMemoryValidationError, match="final_energy"):
        validate_document(document)


def test_loader_rejects_duplicate_and_non_finite_json(tmp_path: Path) -> None:
    duplicate_path = tmp_path / "duplicate.json"
    duplicate_path.write_text('{"id": "a", "id": "b"}', encoding="utf-8")
    with pytest.raises(HopfieldAssociativeMemoryValidationError, match="duplicate"):
        load_json(duplicate_path)

    non_finite_path = tmp_path / "non-finite.json"
    non_finite_path.write_text('{"value": NaN}', encoding="utf-8")
    with pytest.raises(HopfieldAssociativeMemoryValidationError, match="non-finite"):
        load_json(non_finite_path)


def test_operation_contract_is_exact() -> None:
    document = copy.deepcopy(lab())
    document["operation"]["update"] = "parallel"
    with pytest.raises(HopfieldAssociativeMemoryValidationError, match="operation"):
        validate_document(document)


def test_fixture_is_canonical_json() -> None:
    document = lab()
    reparsed = json.loads(json.dumps(document, allow_nan=False))
    assert reparsed == document
