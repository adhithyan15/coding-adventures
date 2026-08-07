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

from validate_backend_parity_labs import (  # noqa: E402
    DEFAULT_FIXTURE_ROOT,
    BackendParityValidationError,
    decode_f32_hex,
    execute_reference,
    load_json,
    validate_document,
    validate_fixture_root,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-dense-batch.json"


def load_lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_checked_in_corpus_and_schema_validate() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(load_lab())
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_hand_calculation_and_f32_paths_match() -> None:
    trace = execute_reference([1, 2, 3], 2, [1, 1, 1])
    assert trace == {
        "products": [2, 4, 6],
        "outputs": [3, 5, 7],
        "f32_products": [2, 4, 6],
        "f32_outputs": [3, 5, 7],
    }


def test_payloads_are_the_same_language_neutral_oracle() -> None:
    inputs = decode_f32_hex(
        (DEFAULT_FIXTURE_ROOT / "payloads" / "00-input-x.f32le.hex").read_text(),
        "inputs",
    )
    outputs = decode_f32_hex(
        (
            DEFAULT_FIXTURE_ROOT / "payloads" / "00-expected-output.f32le.hex"
        ).read_text(),
        "outputs",
    )
    assert inputs == [1, 2, 3]
    assert outputs == [3, 5, 7]


def test_lane_roster_precision_and_residency_are_pinned() -> None:
    document = validate_document(load_lab(), DEFAULT_FIXTURE_ROOT, LAB_PATH)
    assert [lane["id"] for lane in document["lanes"]] == [
        "scalar_cpu",
        "typescript_matrix_cpu",
        "rust_matrix_cpu",
        "webgpu_accelerated",
    ]
    assert [lane["precision"] for lane in document["lanes"]] == [
        "binary64",
        "binary64",
        "f32",
        "f32",
    ]
    assert document["lanes"][2]["residency"][-1] == "host:y bytes"
    assert document["lanes"][3]["availability"] == "optional-runtime-probe"


def test_matrix_ir_is_a_matmul_then_add_with_f32_constants() -> None:
    graph = load_json(DEFAULT_FIXTURE_ROOT / "matrix-ir" / "00-dense-batch.graph.json")
    assert [op["kind"] for op in graph["ops"]] == ["MatMul", "Add"]
    assert graph["inputs"] == [0]
    assert graph["outputs"] == [4]
    assert graph["constants"][0]["bytes_hex"] == "00000040"


def test_rejects_type_coercions_in_canonical_matrix_ir(tmp_path: Path) -> None:
    mutations = [
        lambda graph: graph.__setitem__("matrix_ir_version", True),
        lambda graph: graph["tensors"][0].__setitem__("id", 0.0),
        lambda graph: graph["tensors"][0]["shape"].__setitem__(0, True),
    ]
    for index, mutate in enumerate(mutations):
        root = tmp_path / str(index)
        shutil.copytree(DEFAULT_FIXTURE_ROOT, root)
        graph_path = root / "matrix-ir" / "00-dense-batch.graph.json"
        graph = load_json(graph_path)
        mutate(graph)
        graph_path.write_text(json.dumps(graph), encoding="utf-8")
        with pytest.raises(BackendParityValidationError, match="not canonical"):
            validate_fixture_root(root)


def test_rejects_duplicate_keys_non_finite_numbers_and_unknown_fields(
    tmp_path: Path,
) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}')
    with pytest.raises(BackendParityValidationError, match="duplicate JSON key"):
        load_json(duplicate)

    non_finite = tmp_path / "non-finite.json"
    non_finite.write_text('{"value": NaN}')
    with pytest.raises(BackendParityValidationError, match="non-finite"):
        load_json(non_finite)

    extra = load_lab()
    extra["surprise"] = True
    with pytest.raises(BackendParityValidationError, match="key mismatch"):
        validate_document(extra, DEFAULT_FIXTURE_ROOT, LAB_PATH)


def test_rejects_integer_token_above_python_parser_limit(tmp_path: Path) -> None:
    oversized = tmp_path / "oversized-integer.json"
    oversized.write_text('{"value": ' + "9" * 5000 + "}")
    with pytest.raises(BackendParityValidationError, match="invalid JSON"):
        load_json(oversized)


def test_rejects_json_nesting_above_parser_limit(tmp_path: Path) -> None:
    nested = tmp_path / "deeply-nested.json"
    nested.write_text("[" * 10_000 + "0" + "]" * 10_000)
    with pytest.raises(BackendParityValidationError, match="invalid JSON"):
        load_json(nested)


def test_rejects_path_traversal_and_noncanonical_references() -> None:
    escaped = load_lab()
    escaped["graph"]["matrix_ir_file"] = "../../../../README.md"
    with pytest.raises(BackendParityValidationError, match="canonical dense graph"):
        validate_document(escaped, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    payload = load_lab()
    payload["scenario"]["input_payload_file"] = "../../../../README.md"
    with pytest.raises(BackendParityValidationError, match="not canonical"):
        validate_document(payload, DEFAULT_FIXTURE_ROOT, LAB_PATH)


def test_rejects_wrong_graph_outputs_and_lane_order() -> None:
    graph = load_lab()
    graph["graph"]["weight"] = [3]
    with pytest.raises(BackendParityValidationError, match="canonical dense graph"):
        validate_document(graph, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    output = load_lab()
    output["scenario"]["expected"]["outputs"][1] = 99
    with pytest.raises(BackendParityValidationError, match="expected outputs"):
        validate_document(output, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    lanes = load_lab()
    lanes["lanes"] = list(reversed(lanes["lanes"]))
    with pytest.raises(BackendParityValidationError, match="canonical lane mismatch"):
        validate_document(lanes, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    drift = load_lab()
    drift["scenario"]["expected"]["outputs"][0] = 3.0000009
    for lane in drift["lanes"]:
        lane["expected_outputs"][0] = 3.0000009
    with pytest.raises(BackendParityValidationError, match="expected outputs"):
        validate_document(drift, DEFAULT_FIXTURE_ROOT, LAB_PATH)


def test_rejects_malformed_uppercase_and_nonfinite_f32_payloads() -> None:
    with pytest.raises(BackendParityValidationError, match="lowercase"):
        decode_f32_hex("0000803F", "uppercase")
    with pytest.raises(BackendParityValidationError, match="lowercase"):
        decode_f32_hex("123", "short")
    with pytest.raises(BackendParityValidationError, match="non-finite"):
        decode_f32_hex("0000c07f", "nan")


def test_rejects_non_ascii_payload_as_validation_error(tmp_path: Path) -> None:
    root = tmp_path / "fixture"
    shutil.copytree(DEFAULT_FIXTURE_ROOT, root)
    (root / "payloads" / "00-input-x.f32le.hex").write_bytes(b"\xff")
    with pytest.raises(BackendParityValidationError, match="invalid payload text"):
        validate_fixture_root(root)


def test_rejects_unbounded_inputs_and_noncanonical_tolerance() -> None:
    huge = load_lab()
    huge["scenario"]["inputs"][0] = 10**1000
    with pytest.raises(BackendParityValidationError, match="finite bounded"):
        validate_document(huge, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    loose = load_lab()
    loose["absolute_tolerance"] = 1
    with pytest.raises(BackendParityValidationError, match="canonical"):
        validate_document(loose, DEFAULT_FIXTURE_ROOT, LAB_PATH)

    fractional_shape = load_lab()
    fractional_shape["graph"]["input_shape"] = [3.5, 1]
    with pytest.raises(BackendParityValidationError, match="expected integers"):
        validate_document(fractional_shape, DEFAULT_FIXTURE_ROOT, LAB_PATH)


def test_validated_document_is_a_fresh_normalized_copy() -> None:
    source = load_lab()
    snapshot = copy.deepcopy(source)
    validated = validate_document(source, DEFAULT_FIXTURE_ROOT, LAB_PATH)
    source["scenario"]["inputs"][0] = 999
    assert validated["scenario"]["inputs"] == snapshot["scenario"]["inputs"]
