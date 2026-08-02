#!/usr/bin/env python3
"""Validate the language-neutral NN29 forward graph lowering corpus."""

from __future__ import annotations

import argparse
import heapq
import json
import math
import re
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "forward-graph-lowering-v1"
)

CANONICAL_TOLERANCE = 1e-10
CANONICAL_DOCUMENT_ID = "nn29-forward-graph-lowering"
CANONICAL_GRAPH_NAME = "tiny-weighted-relu"
CANONICAL_EXAMPLE_IDS = ["single_row", "two_row_batch"]
MAX_NODES = 16
MAX_EDGES = 24
MAX_EXAMPLES = 4
MAX_BATCH = 8
MAX_TEXT_LENGTH = 512
MAX_IDENTIFIER_LENGTH = 64
MAX_ABSOLUTE_INPUT = 1e3
MAX_ABSOLUTE_DERIVED = 1e12
MAX_COMPARE_DEPTH = 64

IDENTIFIER = re.compile(r"^[A-Za-z][A-Za-z0-9_]{0,63}$")
DOCUMENT_ID = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
NODE_OPS = {"input", "constant", "weighted_sum", "activation", "output"}
ACTIVATIONS = {"relu", "sigmoid", "tanh", "none"}


class ForwardGraphLoweringValidationError(ValueError):
    """Raised when the NN29 corpus violates its executable contract."""


def _reject_constant(token: str) -> None:
    raise ForwardGraphLoweringValidationError(
        f"non-finite JSON numeric constant: {token}"
    )


def _object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ForwardGraphLoweringValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_object_pairs,
            parse_constant=_reject_constant,
        )
    except (OSError, json.JSONDecodeError) as error:
        raise ForwardGraphLoweringValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise ForwardGraphLoweringValidationError(f"{path}: expected JSON object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ForwardGraphLoweringValidationError(f"{context}: expected object")
    if set(value) != keys:
        raise ForwardGraphLoweringValidationError(
            f"{context}: key mismatch; expected {sorted(keys)}, got {sorted(value)}"
        )
    return value


def _text(value: Any, context: str) -> str:
    if not isinstance(value, str) or not 1 <= len(value) <= MAX_TEXT_LENGTH:
        raise ForwardGraphLoweringValidationError(
            f"{context}: expected non-empty string up to {MAX_TEXT_LENGTH} characters"
        )
    return value


def _identifier(value: Any, context: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) > MAX_IDENTIFIER_LENGTH
        or IDENTIFIER.fullmatch(value) is None
    ):
        raise ForwardGraphLoweringValidationError(
            f"{context}: expected bounded portable identifier"
        )
    return value


def _document_id(value: Any, context: str) -> str:
    text = _text(value, context)
    if len(text) > 128 or DOCUMENT_ID.fullmatch(text) is None:
        raise ForwardGraphLoweringValidationError(
            f"{context}: expected lowercase hyphenated document identifier"
        )
    return text


def _number(value: Any, context: str, *, derived: bool = False) -> float:
    limit = MAX_ABSOLUTE_DERIVED if derived else MAX_ABSOLUTE_INPUT
    valid_type = not isinstance(value, bool) and isinstance(value, (int, float))
    finite_and_bounded = valid_type and abs(value) <= limit
    if finite_and_bounded and isinstance(value, float):
        finite_and_bounded = math.isfinite(value)
    if not finite_and_bounded:
        category = "derived" if derived else "input"
        raise ForwardGraphLoweringValidationError(
            f"{context}: expected finite bounded {category} number"
        )
    return value


def _finite(value: float, context: str) -> float:
    return _number(value, context, derived=True)


def _validate_node(value: Any, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ForwardGraphLoweringValidationError(f"{context}: expected object")
    op = value.get("op")
    if op not in NODE_OPS:
        raise ForwardGraphLoweringValidationError(f"{context}.op: unsupported node op")
    expected_keys = {
        "input": {"id", "op", "input_name"},
        "constant": {"id", "op", "value"},
        "weighted_sum": {"id", "op"},
        "activation": {"id", "op", "activation"},
        "output": {"id", "op", "output_name"},
    }[op]
    node = _object(value, expected_keys, context)
    normalized = {"id": _identifier(node["id"], f"{context}.id"), "op": op}
    if op == "input":
        normalized["input_name"] = _identifier(
            node["input_name"], f"{context}.input_name"
        )
    elif op == "constant":
        normalized["value"] = _number(node["value"], f"{context}.value")
    elif op == "activation":
        activation = node["activation"]
        if activation not in ACTIVATIONS:
            raise ForwardGraphLoweringValidationError(
                f"{context}.activation: unsupported activation"
            )
        normalized["activation"] = activation
    elif op == "output":
        normalized["output_name"] = _identifier(
            node["output_name"], f"{context}.output_name"
        )
    return normalized


def _validate_edge(value: Any, context: str) -> dict[str, Any]:
    edge = _object(value, {"id", "from", "to", "weight"}, context)
    return {
        "id": _identifier(edge["id"], f"{context}.id"),
        "from": _identifier(edge["from"], f"{context}.from"),
        "to": _identifier(edge["to"], f"{context}.to"),
        "weight": _number(edge["weight"], f"{context}.weight"),
    }


def _topological_order(graph: dict[str, Any]) -> list[str]:
    nodes = {node["id"] for node in graph["nodes"]}
    indegree = {node_id: 0 for node_id in nodes}
    outgoing = {node_id: [] for node_id in nodes}
    for edge in graph["edges"]:
        indegree[edge["to"]] += 1
        outgoing[edge["from"]].append(edge["to"])
    ready = [node_id for node_id, degree in indegree.items() if degree == 0]
    heapq.heapify(ready)
    order: list[str] = []
    while ready:
        node_id = heapq.heappop(ready)
        order.append(node_id)
        for destination in sorted(outgoing[node_id]):
            indegree[destination] -= 1
            if indegree[destination] == 0:
                heapq.heappush(ready, destination)
    if len(order) != len(nodes):
        raise ForwardGraphLoweringValidationError("graph: cycle detected")
    return order


def _validate_graph(value: Any) -> dict[str, Any]:
    graph = _object(value, {"name", "nodes", "edges"}, "lab.graph")
    name = _document_id(graph["name"], "lab.graph.name")
    if name != CANONICAL_GRAPH_NAME:
        raise ForwardGraphLoweringValidationError(
            f"lab.graph.name: expected canonical {CANONICAL_GRAPH_NAME}"
        )
    raw_nodes = graph["nodes"]
    raw_edges = graph["edges"]
    if not isinstance(raw_nodes, list) or not 1 <= len(raw_nodes) <= MAX_NODES:
        raise ForwardGraphLoweringValidationError(
            f"lab.graph.nodes: expected 1 to {MAX_NODES} nodes"
        )
    if not isinstance(raw_edges, list) or not 1 <= len(raw_edges) <= MAX_EDGES:
        raise ForwardGraphLoweringValidationError(
            f"lab.graph.edges: expected 1 to {MAX_EDGES} edges"
        )
    nodes = [
        _validate_node(node, f"lab.graph.nodes[{index}]")
        for index, node in enumerate(raw_nodes)
    ]
    edges = [
        _validate_edge(edge, f"lab.graph.edges[{index}]")
        for index, edge in enumerate(raw_edges)
    ]
    node_ids = [node["id"] for node in nodes]
    edge_ids = [edge["id"] for edge in edges]
    if len(set(node_ids)) != len(node_ids):
        raise ForwardGraphLoweringValidationError("lab.graph.nodes: duplicate node id")
    if len(set(edge_ids)) != len(edge_ids):
        raise ForwardGraphLoweringValidationError("lab.graph.edges: duplicate edge id")
    node_id_set = set(node_ids)
    for edge in edges:
        if edge["from"] not in node_id_set or edge["to"] not in node_id_set:
            raise ForwardGraphLoweringValidationError(
                f"edge {edge['id']}: unknown endpoint"
            )
    normalized = {"name": name, "nodes": nodes, "edges": edges}
    _topological_order(normalized)
    nodes_by_id = {node["id"]: node for node in nodes}
    incoming = {node_id: [] for node_id in node_ids}
    for edge in edges:
        incoming[edge["to"]].append(edge)
        if (
            nodes_by_id[edge["to"]]["op"] in {"activation", "output"}
            and edge["weight"] != 1
        ):
            raise ForwardGraphLoweringValidationError(
                f"edge {edge['id']}: connectivity-only edge weight must equal 1"
            )
    for node in nodes:
        count = len(incoming[node["id"]])
        if node["op"] in {"input", "constant"} and count != 0:
            raise ForwardGraphLoweringValidationError(
                f"node {node['id']}: source nodes cannot have incoming edges"
            )
        if node["op"] == "weighted_sum" and count < 1:
            raise ForwardGraphLoweringValidationError(
                f"node {node['id']}: weighted_sum needs an input"
            )
        if node["op"] in {"activation", "output"} and count != 1:
            raise ForwardGraphLoweringValidationError(
                f"node {node['id']}: expected exactly one input edge"
            )
    input_names = [node["input_name"] for node in nodes if node["op"] == "input"]
    output_names = [node["output_name"] for node in nodes if node["op"] == "output"]
    if len(set(input_names)) != len(input_names) or not input_names:
        raise ForwardGraphLoweringValidationError(
            "lab.graph.nodes: input names must be non-empty and unique"
        )
    if len(set(output_names)) != len(output_names) or not output_names:
        raise ForwardGraphLoweringValidationError(
            "lab.graph.nodes: output names must be non-empty and unique"
        )
    return normalized


def compile_neural_ir(graph: dict[str, Any]) -> dict[str, Any]:
    nodes = {node["id"]: node for node in graph["nodes"]}
    incoming = {node_id: [] for node_id in nodes}
    for edge in graph["edges"]:
        incoming[edge["to"]].append(edge)
    values: dict[str, str] = {}
    instructions: list[dict[str, Any]] = []
    next_value = 0

    def allocate() -> str:
        nonlocal next_value
        value_id = f"v{next_value}"
        next_value += 1
        return value_id

    def emit(
        op: str,
        output: str | None,
        inputs: list[str],
        attributes: dict[str, Any],
        source_nodes: list[str],
        source_edges: list[str],
    ) -> None:
        instructions.append(
            {
                "id": f"i{len(instructions)}",
                "op": op,
                "output": output,
                "inputs": inputs,
                "attributes": attributes,
                "source_nodes": source_nodes,
                "source_edges": source_edges,
            }
        )

    for node_id in _topological_order(graph):
        node = nodes[node_id]
        if node["op"] == "input":
            output = allocate()
            values[node_id] = output
            emit(
                "LOAD_INPUT",
                output,
                [],
                {"input_name": node["input_name"]},
                [node_id],
                [],
            )
        elif node["op"] == "constant":
            output = allocate()
            values[node_id] = output
            emit("LOAD_CONST", output, [], {"value": node["value"]}, [node_id], [])
        elif node["op"] == "weighted_sum":
            terms: list[str] = []
            for edge in sorted(incoming[node_id], key=lambda item: item["id"]):
                weight_value = allocate()
                term_value = allocate()
                emit(
                    "LOAD_EDGE_WEIGHT",
                    weight_value,
                    [],
                    {"edge_id": edge["id"]},
                    [],
                    [edge["id"]],
                )
                emit(
                    "MUL",
                    term_value,
                    [values[edge["from"]], weight_value],
                    {},
                    [],
                    [edge["id"]],
                )
                terms.append(term_value)
            output = allocate()
            values[node_id] = output
            emit("ADD", output, terms, {}, [node_id], [])
        elif node["op"] == "activation":
            edge = incoming[node_id][0]
            output = allocate()
            values[node_id] = output
            emit(
                "ACTIVATE",
                output,
                [values[edge["from"]]],
                {"activation": node["activation"]},
                [node_id],
                [],
            )
        else:
            edge = incoming[node_id][0]
            values[node_id] = values[edge["from"]]
            emit(
                "STORE_OUTPUT",
                None,
                [values[edge["from"]]],
                {"output_name": node["output_name"]},
                [node_id],
                [],
            )
    return {"magic": "CANN", "version": 0, "instructions": instructions}


def compile_matrix_ir(
    neural_ir: dict[str, Any], graph: dict[str, Any]
) -> dict[str, Any]:
    edge_weights = {edge["id"]: edge["weight"] for edge in graph["edges"]}
    value_sources: set[str] = set()
    edge_weight_values: dict[str, tuple[str, str]] = {}
    term_values: dict[str, tuple[str, str, list[str]]] = {}
    operations: list[dict[str, Any]] = []

    def emit(
        op: str,
        output: str | None,
        inputs: list[str],
        attributes: dict[str, Any],
        source_instructions: list[str],
        source_nodes: list[str],
        source_edges: list[str],
    ) -> None:
        operations.append(
            {
                "id": f"m{len(operations)}",
                "op": op,
                "output": output,
                "inputs": inputs,
                "attributes": attributes,
                "source_instructions": source_instructions,
                "source_nodes": source_nodes,
                "source_edges": source_edges,
            }
        )

    for instruction in neural_ir["instructions"]:
        op = instruction["op"]
        output = instruction["output"]
        if op == "LOAD_INPUT":
            value_sources.add(output)
            emit(
                "LOAD_INPUT_MATRIX",
                output,
                [],
                dict(instruction["attributes"]),
                [instruction["id"]],
                list(instruction["source_nodes"]),
                [],
            )
        elif op == "LOAD_CONST":
            value_sources.add(output)
            emit(
                "LOAD_CONST_MATRIX",
                output,
                [],
                dict(instruction["attributes"]),
                [instruction["id"]],
                list(instruction["source_nodes"]),
                [],
            )
        elif op == "LOAD_EDGE_WEIGHT":
            edge_id = instruction["attributes"]["edge_id"]
            edge_weight_values[output] = (edge_id, instruction["id"])
        elif op == "MUL":
            left, right = instruction["inputs"]
            if left not in value_sources or right not in edge_weight_values:
                raise ForwardGraphLoweringValidationError(
                    f"{instruction['id']}: MUL is not a lowerable weighted term"
                )
            edge_id, load_id = edge_weight_values[right]
            term_values[output] = (left, edge_id, [load_id, instruction["id"]])
        elif op == "ADD":
            try:
                terms = [term_values[value_id] for value_id in instruction["inputs"]]
            except KeyError as error:
                raise ForwardGraphLoweringValidationError(
                    f"{instruction['id']}: ADD input is not a weighted term"
                ) from error
            inputs = [term[0] for term in terms]
            edge_ids = [term[1] for term in terms]
            source_ids = [source_id for term in terms for source_id in term[2]] + [
                instruction["id"]
            ]
            value_sources.add(output)
            emit(
                "WEIGHTED_SUM_MATRIX",
                output,
                inputs,
                {
                    "edge_ids": edge_ids,
                    "weights": [edge_weights[edge_id] for edge_id in edge_ids],
                },
                source_ids,
                list(instruction["source_nodes"]),
                edge_ids,
            )
        elif op == "ACTIVATE":
            if instruction["inputs"][0] not in value_sources:
                raise ForwardGraphLoweringValidationError(
                    f"{instruction['id']}: activation input is unavailable"
                )
            value_sources.add(output)
            emit(
                "ACTIVATE_MATRIX",
                output,
                list(instruction["inputs"]),
                dict(instruction["attributes"]),
                [instruction["id"]],
                list(instruction["source_nodes"]),
                [],
            )
        elif op == "STORE_OUTPUT":
            if instruction["inputs"][0] not in value_sources:
                raise ForwardGraphLoweringValidationError(
                    f"{instruction['id']}: output input is unavailable"
                )
            emit(
                "STORE_OUTPUT_MATRIX",
                None,
                list(instruction["inputs"]),
                dict(instruction["attributes"]),
                [instruction["id"]],
                list(instruction["source_nodes"]),
                [],
            )
    return {
        "magic": "CANM",
        "version": 0,
        "source_neural_ir_version": neural_ir["version"],
        "operations": operations,
    }


def _activate(value: float, activation: str, context: str) -> float:
    if activation == "relu":
        return max(0.0, value)
    if activation == "sigmoid":
        clipped = max(-500.0, min(500.0, value))
        return _finite(1.0 / (1.0 + math.exp(-clipped)), context)
    if activation == "tanh":
        return math.tanh(value)
    return value


def _validate_input_columns(
    value: Any, graph: dict[str, Any], context: str
) -> dict[str, list[float]]:
    if not isinstance(value, dict):
        raise ForwardGraphLoweringValidationError(f"{context}: expected object")
    input_names = sorted(
        node["input_name"] for node in graph["nodes"] if node["op"] == "input"
    )
    if sorted(value) != input_names:
        raise ForwardGraphLoweringValidationError(
            f"{context}: input names expected {input_names}, got {sorted(value)}"
        )
    columns: dict[str, list[float]] = {}
    batch_size: int | None = None
    for input_name in input_names:
        column = value[input_name]
        if not isinstance(column, list) or not 1 <= len(column) <= MAX_BATCH:
            raise ForwardGraphLoweringValidationError(
                f"{context}.{input_name}: expected 1 to {MAX_BATCH} values"
            )
        if batch_size is None:
            batch_size = len(column)
        elif len(column) != batch_size:
            raise ForwardGraphLoweringValidationError(
                f"{context}: every input column must have the same length"
            )
        columns[input_name] = [
            _number(item, f"{context}.{input_name}[{index}]")
            for index, item in enumerate(column)
        ]
    return columns


def execute_graph(graph: dict[str, Any], inputs: dict[str, list[float]]) -> list[float]:
    nodes = {node["id"]: node for node in graph["nodes"]}
    incoming = {node_id: [] for node_id in nodes}
    for edge in graph["edges"]:
        incoming[edge["to"]].append(edge)
    for edges in incoming.values():
        edges.sort(key=lambda edge: edge["id"])
    outputs: list[float] = []
    batch_size = len(next(iter(inputs.values())))
    for row in range(batch_size):
        values: dict[str, float] = {}
        named_outputs: dict[str, float] = {}
        for node_id in _topological_order(graph):
            node = nodes[node_id]
            if node["op"] == "input":
                values[node_id] = inputs[node["input_name"]][row]
            elif node["op"] == "constant":
                values[node_id] = node["value"]
            elif node["op"] == "weighted_sum":
                values[node_id] = _finite(
                    sum(
                        _finite(
                            values[edge["from"]] * edge["weight"],
                            f"graph row {row} edge {edge['id']}",
                        )
                        for edge in incoming[node_id]
                    ),
                    f"graph row {row} node {node_id}",
                )
            elif node["op"] == "activation":
                source = incoming[node_id][0]["from"]
                values[node_id] = _activate(
                    values[source], node["activation"], f"graph row {row} activation"
                )
            else:
                source = incoming[node_id][0]["from"]
                values[node_id] = values[source]
                named_outputs[node["output_name"]] = values[source]
        if set(named_outputs) != {"prediction"}:
            raise ForwardGraphLoweringValidationError(
                "graph: canonical lab must publish only prediction"
            )
        outputs.append(named_outputs["prediction"])
    return outputs


def execute_neural_ir(
    neural_ir: dict[str, Any],
    graph: dict[str, Any],
    inputs: dict[str, list[float]],
) -> tuple[list[float], list[list[float]]]:
    edge_weights = {edge["id"]: edge["weight"] for edge in graph["edges"]}
    outputs: list[float] = []
    value_rows: list[list[float]] = []
    batch_size = len(next(iter(inputs.values())))
    for row in range(batch_size):
        values: dict[str, float] = {}
        output: float | None = None
        for instruction in neural_ir["instructions"]:
            op = instruction["op"]
            destination = instruction["output"]
            if op == "LOAD_INPUT":
                value = inputs[instruction["attributes"]["input_name"]][row]
            elif op == "LOAD_CONST":
                value = instruction["attributes"]["value"]
            elif op == "LOAD_EDGE_WEIGHT":
                value = edge_weights[instruction["attributes"]["edge_id"]]
            elif op == "MUL":
                value = _finite(
                    values[instruction["inputs"][0]] * values[instruction["inputs"][1]],
                    f"neural row {row} {instruction['id']}",
                )
            elif op == "ADD":
                value = _finite(
                    sum(values[value_id] for value_id in instruction["inputs"]),
                    f"neural row {row} {instruction['id']}",
                )
            elif op == "ACTIVATE":
                value = _activate(
                    values[instruction["inputs"][0]],
                    instruction["attributes"]["activation"],
                    f"neural row {row} {instruction['id']}",
                )
            else:
                output = values[instruction["inputs"][0]]
                continue
            if destination is None:
                raise ForwardGraphLoweringValidationError(
                    f"{instruction['id']}: value-producing instruction has no output"
                )
            values[destination] = _finite(
                value, f"neural row {row} {instruction['id']} output"
            )
        if output is None:
            raise ForwardGraphLoweringValidationError("neural IR: no output stored")
        outputs.append(output)
        value_rows.append(list(values.values()))
    return outputs, value_rows


def execute_matrix_ir(
    matrix_ir: dict[str, Any], inputs: dict[str, list[float]]
) -> tuple[list[float], list[dict[str, Any]]]:
    batch_size = len(next(iter(inputs.values())))
    values: dict[str, list[float]] = {}
    output: list[float] | None = None
    for operation in matrix_ir["operations"]:
        op = operation["op"]
        destination = operation["output"]
        if op == "LOAD_INPUT_MATRIX":
            column = list(inputs[operation["attributes"]["input_name"]])
        elif op == "LOAD_CONST_MATRIX":
            column = [operation["attributes"]["value"]] * batch_size
        elif op == "WEIGHTED_SUM_MATRIX":
            weights = operation["attributes"]["weights"]
            if len(weights) != len(operation["inputs"]):
                raise ForwardGraphLoweringValidationError(
                    f"{operation['id']}: term and weight counts differ"
                )
            column = [
                _finite(
                    sum(
                        _finite(
                            values[value_id][row] * weights[index],
                            f"matrix {operation['id']} row {row} term {index}",
                        )
                        for index, value_id in enumerate(operation["inputs"])
                    ),
                    f"matrix {operation['id']} row {row}",
                )
                for row in range(batch_size)
            ]
        elif op == "ACTIVATE_MATRIX":
            column = [
                _activate(
                    value,
                    operation["attributes"]["activation"],
                    f"matrix {operation['id']} activation",
                )
                for value in values[operation["inputs"][0]]
            ]
        else:
            output = list(values[operation["inputs"][0]])
            continue
        if destination is None:
            raise ForwardGraphLoweringValidationError(
                f"{operation['id']}: value-producing operation has no output"
            )
        values[destination] = [
            _finite(value, f"matrix {operation['id']} output") for value in column
        ]
    if output is None:
        raise ForwardGraphLoweringValidationError("matrix IR: no output stored")
    columns = [
        {"value_id": value_id, "values": column} for value_id, column in values.items()
    ]
    return output, columns


def _compare(
    actual: Any,
    expected: Any,
    tolerance: float,
    context: str,
    depth: int = 0,
) -> None:
    if depth > MAX_COMPARE_DEPTH:
        raise ForwardGraphLoweringValidationError(
            f"{context}: comparison nesting exceeds {MAX_COMPARE_DEPTH}"
        )
    if isinstance(expected, bool):
        if actual is not expected:
            raise ForwardGraphLoweringValidationError(f"{context}: value mismatch")
    elif isinstance(expected, (int, float)):
        expected_number = _number(expected, f"{context} expected", derived=True)
        actual_number = _number(actual, context, derived=True)
        if abs(actual_number - expected_number) > tolerance:
            raise ForwardGraphLoweringValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str) or expected is None:
        if actual != expected:
            raise ForwardGraphLoweringValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise ForwardGraphLoweringValidationError(f"{context}: list mismatch")
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]", depth + 1)
    elif isinstance(expected, dict):
        if not isinstance(actual, dict) or actual.keys() != expected.keys():
            actual_keys = sorted(actual) if isinstance(actual, dict) else []
            raise ForwardGraphLoweringValidationError(
                f"{context}: object keys expected {sorted(expected)}, got {actual_keys}"
            )
        for key, expected_value in expected.items():
            _compare(
                actual[key],
                expected_value,
                tolerance,
                f"{context}.{key}",
                depth + 1,
            )
    else:
        raise ForwardGraphLoweringValidationError(
            f"{context}: unsupported expected value"
        )


def execute_example(
    graph: dict[str, Any],
    neural_ir: dict[str, Any],
    matrix_ir: dict[str, Any],
    inputs: dict[str, list[float]],
) -> dict[str, Any]:
    direct_outputs = execute_graph(graph, inputs)
    neural_outputs, neural_value_rows = execute_neural_ir(neural_ir, graph, inputs)
    matrix_outputs, matrix_value_columns = execute_matrix_ir(matrix_ir, inputs)
    return {
        "direct_outputs": direct_outputs,
        "neural_ir_outputs": neural_outputs,
        "matrix_ir_outputs": matrix_outputs,
        "neural_value_rows": neural_value_rows,
        "matrix_value_columns": matrix_value_columns,
    }


def validate_document(document: dict[str, Any]) -> dict[str, Any]:
    lab = _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "graph",
            "expected_neural_ir",
            "expected_matrix_ir",
            "examples",
        },
        "lab",
    )
    if lab["schema_version"] != 1:
        raise ForwardGraphLoweringValidationError("lab.schema_version: expected 1")
    lab_id = _document_id(lab["id"], "lab.id")
    if lab_id != CANONICAL_DOCUMENT_ID:
        raise ForwardGraphLoweringValidationError(
            f"lab.id: expected canonical {CANONICAL_DOCUMENT_ID}"
        )
    _text(lab["title"], "lab.title")
    _text(lab["question"], "lab.question")
    tolerance = _number(lab["absolute_tolerance"], "lab.absolute_tolerance")
    if tolerance != CANONICAL_TOLERANCE:
        raise ForwardGraphLoweringValidationError(
            f"lab.absolute_tolerance: expected canonical {CANONICAL_TOLERANCE}"
        )
    graph = _validate_graph(lab["graph"])
    neural_ir = compile_neural_ir(graph)
    matrix_ir = compile_matrix_ir(neural_ir, graph)
    _compare(neural_ir, lab["expected_neural_ir"], 0.0, "lab.expected_neural_ir")
    _compare(matrix_ir, lab["expected_matrix_ir"], 0.0, "lab.expected_matrix_ir")

    raw_examples = lab["examples"]
    if not isinstance(raw_examples, list) or not 1 <= len(raw_examples) <= MAX_EXAMPLES:
        raise ForwardGraphLoweringValidationError(
            f"lab.examples: expected 1 to {MAX_EXAMPLES} examples"
        )
    examples: list[dict[str, Any]] = []
    for index, raw_example in enumerate(raw_examples):
        context = f"lab.examples[{index}]"
        example = _object(raw_example, {"id", "title", "inputs", "expected"}, context)
        example_id = _identifier(example["id"], f"{context}.id")
        title = _text(example["title"], f"{context}.title")
        inputs = _validate_input_columns(example["inputs"], graph, f"{context}.inputs")
        if not isinstance(example["expected"], dict):
            raise ForwardGraphLoweringValidationError(
                f"{context}.expected: expected object"
            )
        actual = execute_example(graph, neural_ir, matrix_ir, inputs)
        _compare(actual, example["expected"], tolerance, f"{context}.expected")
        for row, (direct, neural, matrix) in enumerate(
            zip(
                actual["direct_outputs"],
                actual["neural_ir_outputs"],
                actual["matrix_ir_outputs"],
            )
        ):
            if (
                max(abs(direct - neural), abs(direct - matrix), abs(neural - matrix))
                > tolerance
            ):
                raise ForwardGraphLoweringValidationError(
                    f"{context}: execution parity failed at row {row}"
                )
        examples.append(
            {"id": example_id, "title": title, "inputs": inputs, "expected": actual}
        )
    example_ids = [example["id"] for example in examples]
    if example_ids != CANONICAL_EXAMPLE_IDS:
        raise ForwardGraphLoweringValidationError(
            f"lab.examples: ids expected {CANONICAL_EXAMPLE_IDS}, got {example_ids}"
        )
    return {
        **lab,
        "graph": graph,
        "expected_neural_ir": neural_ir,
        "expected_matrix_ir": matrix_ir,
        "examples": examples,
    }


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise ForwardGraphLoweringValidationError(f"{root}: no lab JSON files")
    for path in paths:
        validate_document(load_json(path))
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    try:
        count = validate_fixture_root(args.root)
    except ForwardGraphLoweringValidationError as error:
        parser.error(str(error))
    print(f"validated {count} forward graph lowering lab(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
