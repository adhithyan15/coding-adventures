from __future__ import annotations

import copy
import json
import shutil
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_precision_residency_labs import (
    DEFAULT_FIXTURE_ROOT,
    PrecisionResidencyValidationError,
    decode_hex,
    execute_float_reference,
    execute_int8_reference,
    load_json,
    round_binary16,
    round_binary32,
    validate_document,
    validate_fixture_root,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-tiny-affine.json"


def load_lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_checked_in_corpus_and_schema_validate() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(load_lab())
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_float_rounding_reproduces_the_hand_example() -> None:
    inputs = [1.0004, 1.0006]
    f32 = execute_float_reference(inputs, 2, 0, round_binary32)
    f16 = execute_float_reference(inputs, 2, 0, round_binary16)
    assert f32["encoded_inputs"] == [1.0003999471664429, 1.000599980354309]
    assert f32["outputs"] == [2.0007998943328857, 2.001199960708618]
    assert f16["encoded_inputs"] == [1, 1.0009765625]
    assert f16["outputs"] == [2, 2.001953125]


def test_int8_quantization_reproduces_integer_accumulators() -> None:
    assert execute_int8_reference([1.0004, 1.0006], 2, 0.01, 0.5) == {
        "encoded_inputs": [100, 100],
        "encoded_weight": 4,
        "accumulators": [400, 400],
        "outputs": [2, 2],
    }

    ties = execute_int8_reference([-0.005, 0.005, -0.015, 0.015], 0.5, 0.01, 0.5)
    assert ties["encoded_inputs"] == [0, 0, -2, 2]


def test_language_neutral_payloads_decode_to_the_oracles() -> None:
    payload = DEFAULT_FIXTURE_ROOT / "payloads"
    assert decode_hex(payload / "00-input-x.f32le.hex", 4, False, "f32") == [
        1.0003999471664429,
        1.000599980354309,
    ]
    assert decode_hex(payload / "00-output-y.f16le.hex", 2, False, "f16") == [
        2,
        2.001953125,
    ]
    assert decode_hex(payload / "00-input-x.i8.hex", 1, True, "int8") == [100, 100]


def test_transfer_counts_distinguish_eager_and_resident_buffers() -> None:
    validated = validate_document(load_lab(), DEFAULT_FIXTURE_ROOT, LAB_PATH)
    strategies = validated["residency"]["strategies"]
    assert [(item["id"], item["total_transfer_bytes"]) for item in strategies] == [
        ("eager", 72),
        ("resident", 24),
    ]


def test_rejects_duplicate_keys_non_finite_numbers_and_unknown_fields(
    tmp_path: Path,
) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}')
    with pytest.raises(PrecisionResidencyValidationError, match="duplicate JSON key"):
        load_json(duplicate)

    non_finite = tmp_path / "non-finite.json"
    non_finite.write_text('{"value": NaN}')
    with pytest.raises(PrecisionResidencyValidationError, match="non-finite"):
        load_json(non_finite)

    extra = load_lab()
    extra["surprise"] = True
    with pytest.raises(PrecisionResidencyValidationError, match="key mismatch"):
        validate_document(extra, DEFAULT_FIXTURE_ROOT, LAB_PATH)


def test_rejects_oversized_integer_and_deep_json(tmp_path: Path) -> None:
    oversized = tmp_path / "oversized.json"
    oversized.write_text('{"value": ' + "9" * 5000 + "}")
    with pytest.raises(PrecisionResidencyValidationError, match="invalid JSON"):
        load_json(oversized)

    nested = tmp_path / "nested.json"
    nested.write_text("[" * 10_000 + "0" + "]" * 10_000)
    with pytest.raises(PrecisionResidencyValidationError, match="invalid JSON"):
        load_json(nested)


def test_rejects_changed_rounding_quantization_and_error_oracles() -> None:
    rounded = load_lab()
    rounded["formats"][1]["encoded_inputs"][0] = 1.0004
    with pytest.raises(PrecisionResidencyValidationError, match="dishonest"):
        validate_document(rounded, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    scale = load_lab()
    scale["formats"][2]["input_scale"] = 0.02
    with pytest.raises(PrecisionResidencyValidationError, match="scales"):
        validate_document(scale, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    error = load_lab()
    error["formats"][0]["maximum_absolute_error"] = 0
    with pytest.raises(PrecisionResidencyValidationError, match="dishonest"):
        validate_document(error, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    accumulator_width = load_lab()
    accumulator_width["formats"][2]["accumulator_storage_bytes"] = 1
    with pytest.raises(PrecisionResidencyValidationError, match="int8 contract"):
        validate_document(accumulator_width, DEFAULT_FIXTURE_ROOT, LAB_PATH)


def test_rejects_dishonest_format_and_residency_narrative() -> None:
    title = load_lab()
    title["formats"][1]["title"] = "IEEE-754 binary64"
    with pytest.raises(PrecisionResidencyValidationError, match="title"):
        validate_document(title, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    strategy_title = load_lab()
    strategy_title["residency"]["strategies"][0]["title"] = "Resident buffers"
    with pytest.raises(PrecisionResidencyValidationError, match="narrative"):
        validate_document(strategy_title, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    steps = load_lab()
    steps["residency"]["strategies"][1]["steps"][1] = "download never"
    with pytest.raises(PrecisionResidencyValidationError, match="narrative"):
        validate_document(steps, DEFAULT_FIXTURE_ROOT, LAB_PATH)


def test_rejects_path_traversal_malformed_payload_and_wrong_transfer_total(
    tmp_path: Path,
) -> None:
    escaped = load_lab()
    escaped["formats"][0]["input_payload_file"] = "../../../../README.md"
    with pytest.raises(PrecisionResidencyValidationError, match="suffix|escapes"):
        validate_document(escaped, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    root = tmp_path / "fixture"
    shutil.copytree(DEFAULT_FIXTURE_ROOT, root)
    (root / "payloads" / "00-input-x.f16le.hex").write_text("003C013C")
    with pytest.raises(PrecisionResidencyValidationError, match="lowercase"):
        validate_fixture_root(root)

    transfers = load_lab()
    transfers["residency"]["strategies"][1]["total_transfer_bytes"] = 23
    with pytest.raises(PrecisionResidencyValidationError, match="transfer oracle"):
        validate_document(transfers, DEFAULT_FIXTURE_ROOT, LAB_PATH)


def test_rejects_non_ascii_payload_and_extra_fixture_file(tmp_path: Path) -> None:
    root = tmp_path / "fixture"
    shutil.copytree(DEFAULT_FIXTURE_ROOT, root)
    (root / "payloads" / "00-input-x.i8.hex").write_bytes(b"\xff")
    with pytest.raises(PrecisionResidencyValidationError, match="invalid payload text"):
        validate_fixture_root(root)

    root = tmp_path / "fixture-extra"
    shutil.copytree(DEFAULT_FIXTURE_ROOT, root)
    (root / "surprise.txt").write_text("extra")
    with pytest.raises(PrecisionResidencyValidationError, match="file roster"):
        validate_fixture_root(root)


def test_validated_document_is_a_fresh_copy() -> None:
    source = load_lab()
    snapshot = copy.deepcopy(source)
    validated = validate_document(source, DEFAULT_FIXTURE_ROOT, LAB_PATH)
    source["scenario"]["inputs"][0] = 999
    assert validated["scenario"]["inputs"] == snapshot["scenario"]["inputs"]


def test_rejects_unbounded_values_and_signed_int8_overflow() -> None:
    huge = load_lab()
    huge["scenario"]["inputs"][0] = 10**1000
    with pytest.raises(PrecisionResidencyValidationError, match="finite bounded"):
        validate_document(huge, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    with pytest.raises(PrecisionResidencyValidationError, match="signed range"):
        execute_int8_reference([2], 2, 0.01, 0.5)


def test_json_round_trip_preserves_canonical_document() -> None:
    source = load_lab()
    assert json.loads(json.dumps(source)) == source
