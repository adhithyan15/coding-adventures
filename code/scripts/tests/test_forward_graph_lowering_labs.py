from __future__ import annotations

import copy
import math
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_forward_graph_lowering_labs import (
    DEFAULT_FIXTURE_ROOT,
    ForwardGraphLoweringValidationError,
    _compare,
    compile_matrix_ir,
    compile_neural_ir,
    execute_example,
    execute_graph,
    load_json,
    validate_document,
    validate_fixture_root,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-tiny-weighted-relu.json"


def load_lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_checked_in_corpus_and_schema_validate() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(load_lab())
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_graph_lowers_to_the_exact_neural_instruction_stream() -> None:
    document = validate_document(load_lab())
    neural_ir = compile_neural_ir(document["graph"])

    assert neural_ir["magic"] == "CANN"
    assert [instruction["id"] for instruction in neural_ir["instructions"]] == [
        f"i{index}" for index in range(12)
    ]
    assert [instruction["op"] for instruction in neural_ir["instructions"]] == [
        "LOAD_CONST",
        "LOAD_INPUT",
        "LOAD_INPUT",
        "LOAD_EDGE_WEIGHT",
        "MUL",
        "LOAD_EDGE_WEIGHT",
        "MUL",
        "LOAD_EDGE_WEIGHT",
        "MUL",
        "ADD",
        "ACTIVATE",
        "STORE_OUTPUT",
    ]
    assert neural_ir == document["expected_neural_ir"]


def test_matrix_lowering_fuses_weight_loads_products_and_addition() -> None:
    document = validate_document(load_lab())
    matrix_ir = compile_matrix_ir(document["expected_neural_ir"], document["graph"])

    assert [operation["op"] for operation in matrix_ir["operations"]] == [
        "LOAD_CONST_MATRIX",
        "LOAD_INPUT_MATRIX",
        "LOAD_INPUT_MATRIX",
        "WEIGHTED_SUM_MATRIX",
        "ACTIVATE_MATRIX",
        "STORE_OUTPUT_MATRIX",
    ]
    fused = matrix_ir["operations"][3]
    assert fused["source_instructions"] == [
        "i3",
        "i4",
        "i5",
        "i6",
        "i7",
        "i8",
        "i9",
    ]
    assert fused["attributes"] == {
        "edge_ids": ["bias_to_sum", "w0", "w1"],
        "weights": [-1, 0.25, 0.75],
    }
    assert matrix_ir == document["expected_matrix_ir"]


def test_one_row_replays_every_hand_calculated_value() -> None:
    document = validate_document(load_lab())
    example = document["examples"][0]
    trace = execute_example(
        document["graph"],
        document["expected_neural_ir"],
        document["expected_matrix_ir"],
        example["inputs"],
    )

    assert trace["direct_outputs"] == [6]
    assert trace["neural_ir_outputs"] == [6]
    assert trace["matrix_ir_outputs"] == [6]
    assert trace["neural_value_rows"] == [[1, 4, 8, -1, -1, 0.25, 1, 0.75, 6, 6, 6]]
    assert trace["matrix_value_columns"][-2:] == [
        {"value_id": "v9", "values": [6]},
        {"value_id": "v10", "values": [6]},
    ]


def test_the_same_lowered_program_runs_a_two_row_batch() -> None:
    document = validate_document(load_lab())
    example = document["examples"][1]
    trace = execute_example(
        document["graph"],
        document["expected_neural_ir"],
        document["expected_matrix_ir"],
        example["inputs"],
    )

    assert trace["direct_outputs"] == [6, 13]
    assert trace["neural_ir_outputs"] == [6, 13]
    assert trace["matrix_ir_outputs"] == [6, 13]
    assert trace["matrix_value_columns"][3] == {
        "value_id": "v9",
        "values": [6, 13],
    }


def test_lowering_is_independent_of_json_node_and_edge_record_order() -> None:
    reordered = load_lab()
    reordered["graph"]["nodes"] = list(reversed(reordered["graph"]["nodes"]))
    reordered["graph"]["edges"] = list(reversed(reordered["graph"]["edges"]))

    document = validate_document(reordered)
    assert document["expected_neural_ir"] == load_lab()["expected_neural_ir"]
    assert document["expected_matrix_ir"] == load_lab()["expected_matrix_ir"]


def test_direct_execution_uses_the_same_stable_edge_order_as_lowering() -> None:
    graph = validate_document(load_lab())["graph"]
    weights = {"bias_to_sum": 1e-12, "w0": 1000, "w1": -1000}
    for edge in graph["edges"]:
        if edge["id"] in weights:
            edge["weight"] = weights[edge["id"]]
    inputs = {"x0": [1000], "x1": [1000]}

    expected = execute_graph(graph, inputs)
    graph["edges"] = list(reversed(graph["edges"]))

    assert expected == [0]
    assert execute_graph(graph, inputs) == expected


def test_rejects_duplicate_keys_non_finite_numbers_and_unknown_fields(
    tmp_path: Path,
) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(ForwardGraphLoweringValidationError, match="duplicate JSON key"):
        load_json(duplicate)

    non_finite = tmp_path / "non-finite.json"
    non_finite.write_text('{"value": NaN}', encoding="utf-8")
    with pytest.raises(ForwardGraphLoweringValidationError, match="non-finite"):
        load_json(non_finite)

    extra = load_lab()
    extra["surprise"] = True
    with pytest.raises(ForwardGraphLoweringValidationError, match="key mismatch"):
        validate_document(extra)


def test_rejects_dangling_edges_cycles_and_unsupported_activations() -> None:
    dangling = load_lab()
    dangling["graph"]["edges"][0]["from"] = "ghost"
    with pytest.raises(ForwardGraphLoweringValidationError, match="unknown endpoint"):
        validate_document(dangling)

    cyclic = load_lab()
    cyclic["graph"]["edges"][0]["from"] = "relu"
    with pytest.raises(ForwardGraphLoweringValidationError, match="cycle"):
        validate_document(cyclic)

    unsupported = load_lab()
    unsupported["graph"]["nodes"][4]["activation"] = "gelu"
    with pytest.raises(
        ForwardGraphLoweringValidationError, match="unsupported activation"
    ):
        validate_document(unsupported)

    weighted_passthrough = load_lab()
    weighted_passthrough["graph"]["edges"][3]["weight"] = 2
    with pytest.raises(
        ForwardGraphLoweringValidationError, match="connectivity-only edge weight"
    ):
        validate_document(weighted_passthrough)


def test_rejects_loose_tolerance_dishonest_ir_and_dishonest_outputs() -> None:
    loose = load_lab()
    loose["absolute_tolerance"] = 1
    with pytest.raises(ForwardGraphLoweringValidationError, match="canonical"):
        validate_document(loose)

    dishonest_ir = load_lab()
    dishonest_ir["expected_neural_ir"]["instructions"][9]["inputs"] = ["v4"]
    with pytest.raises(ForwardGraphLoweringValidationError, match="list mismatch"):
        validate_document(dishonest_ir)

    dishonest_output = load_lab()
    dishonest_output["examples"][0]["expected"]["matrix_ir_outputs"] = [999]
    with pytest.raises(ForwardGraphLoweringValidationError, match="expected 999"):
        validate_document(dishonest_output)


def test_rejects_mismatched_batches_large_inputs_and_bad_example_roster() -> None:
    mismatched = load_lab()
    mismatched["examples"][1]["inputs"]["x1"] = [8]
    with pytest.raises(ForwardGraphLoweringValidationError, match="same length"):
        validate_document(mismatched)

    huge = load_lab()
    huge["examples"][0]["inputs"]["x0"] = [int("9" * 1000)]
    with pytest.raises(ForwardGraphLoweringValidationError, match="finite bounded"):
        validate_document(huge)

    reordered = load_lab()
    reordered["examples"] = list(reversed(reordered["examples"]))
    with pytest.raises(ForwardGraphLoweringValidationError, match="ids expected"):
        validate_document(reordered)


def test_comparison_rejects_adversarial_nesting() -> None:
    deeply_nested: object = 0
    for _ in range(66):
        deeply_nested = [deeply_nested]

    with pytest.raises(ForwardGraphLoweringValidationError, match="nesting exceeds"):
        _compare(deeply_nested, deeply_nested, 0.0, "hostile")


def test_every_derived_execution_value_is_finite() -> None:
    document = validate_document(load_lab())
    for example in document["examples"]:
        trace = execute_example(
            document["graph"],
            document["expected_neural_ir"],
            document["expected_matrix_ir"],
            example["inputs"],
        )
        assert all(
            math.isfinite(value)
            for key in ("direct_outputs", "neural_ir_outputs", "matrix_ir_outputs")
            for value in trace[key]
        )
        assert all(
            math.isfinite(value) for row in trace["neural_value_rows"] for value in row
        )
        assert all(
            math.isfinite(value)
            for column in trace["matrix_value_columns"]
            for value in column["values"]
        )


def test_validated_document_is_a_fresh_normalized_copy() -> None:
    source = load_lab()
    snapshot = copy.deepcopy(source)
    validated = validate_document(source)
    source["graph"]["edges"][0]["weight"] = 999

    assert (
        validated["graph"]["edges"][0]["weight"]
        == snapshot["graph"]["edges"][0]["weight"]
    )
