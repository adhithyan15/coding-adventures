#!/usr/bin/env python3
"""Validate and execute the deterministic NN21 tiny message-passing corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "tiny-message-passing-v1"
)


class TinyMessagePassingValidationError(ValueError):
    """Raised when an NN21 document or trace is invalid."""


def _reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise TinyMessagePassingValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicates,
            parse_constant=lambda item: (_ for _ in ()).throw(
                TinyMessagePassingValidationError(f"non-finite JSON number: {item}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise TinyMessagePassingValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise TinyMessagePassingValidationError(
            "top-level JSON value must be an object"
        )
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise TinyMessagePassingValidationError(f"{context}: expected an object")
    missing, extra = keys - value.keys(), value.keys() - keys
    if missing or extra:
        raise TinyMessagePassingValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(float(value))
    ):
        raise TinyMessagePassingValidationError(f"{context}: expected a finite number")
    return float(value)


def _integer(value: Any, context: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TinyMessagePassingValidationError(f"{context}: expected an integer")
    return value


def _zero(value: float) -> float:
    return 0.0 if abs(value) < 1e-12 else value


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    raw_features = document["node_features"]
    if not isinstance(raw_features, list) or len(raw_features) < 2:
        raise TinyMessagePassingValidationError(
            "node_features: expected at least two values"
        )
    features = [
        _number(value, f"node_features[{index}]")
        for index, value in enumerate(raw_features)
    ]
    parameters = _object(
        document["parameters"], {"message_weight", "self_weight", "bias"}, "parameters"
    )
    message_weight = _number(parameters["message_weight"], "parameters.message_weight")
    self_weight = _number(parameters["self_weight"], "parameters.self_weight")
    bias = _number(parameters["bias"], "parameters.bias")
    raw_edges = document["edges"]
    if not isinstance(raw_edges, list) or not raw_edges:
        raise TinyMessagePassingValidationError("edges: expected a non-empty array")
    edges: list[tuple[int, int]] = []
    keys: set[tuple[int, int]] = set()
    for index, raw_edge in enumerate(raw_edges):
        edge = _object(raw_edge, {"source", "target"}, f"edges[{index}]")
        source = _integer(edge["source"], f"edges[{index}].source")
        target = _integer(edge["target"], f"edges[{index}].target")
        if (
            source == target
            or min(source, target) < 0
            or max(source, target) >= len(features)
        ):
            raise TinyMessagePassingValidationError(
                f"edges[{index}]: invalid non-self edge"
            )
        key = (min(source, target), max(source, target))
        if key in keys:
            raise TinyMessagePassingValidationError("edges: duplicate undirected edge")
        keys.add(key)
        edges.append((source, target))

    directed: list[dict[str, Any]] = []
    for source, target in edges:
        for directed_source, directed_target in ((source, target), (target, source)):
            directed.append(
                {
                    "source": directed_source,
                    "target": directed_target,
                    "source_feature": features[directed_source],
                    "message_weight": message_weight,
                    "message": _zero(message_weight * features[directed_source]),
                }
            )
    directed.sort(key=lambda row: (row["target"], row["source"]))
    updates = []
    for node, old_feature in enumerate(features):
        incoming = [row for row in directed if row["target"] == node]
        if not incoming:
            raise TinyMessagePassingValidationError(
                f"node {node} has no incoming messages"
            )
        aggregate = _zero(sum(row["message"] for row in incoming))
        self_contribution = _zero(self_weight * old_feature)
        preactivation = _zero(self_contribution + aggregate + bias)
        updates.append(
            {
                "node": node,
                "old_feature": old_feature,
                "incoming": incoming,
                "aggregate": aggregate,
                "self_contribution": self_contribution,
                "bias": bias,
                "preactivation": preactivation,
                "output_feature": max(0.0, preactivation),
            }
        )
    return {
        "directed_messages": directed,
        "node_updates": updates,
        "output_features": [row["output_feature"] for row in updates],
    }


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise TinyMessagePassingValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, (int, float)):
        if abs(_number(actual, context) - float(expected)) > tolerance:
            raise TinyMessagePassingValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str):
        if actual != expected:
            raise TinyMessagePassingValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise TinyMessagePassingValidationError(f"{context}: array length mismatch")
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]")
    elif isinstance(expected, dict):
        value = _object(actual, set(expected), context)
        for key, right in expected.items():
            _compare(value[key], right, tolerance, f"{context}.{key}")
    else:
        raise TinyMessagePassingValidationError(
            f"{context}: unsupported expected value"
        )


def validate_document(document: dict[str, Any]) -> dict[str, Any]:
    root = _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "concepts",
            "operation",
            "node_features",
            "edges",
            "parameters",
            "expected",
        },
        "document",
    )
    if _integer(root["schema_version"], "schema_version") != 1:
        raise TinyMessagePassingValidationError("schema_version must be 1")
    for key in ("id", "title", "question"):
        if not isinstance(root[key], str) or not root[key].strip():
            raise TinyMessagePassingValidationError(
                f"{key}: expected a non-empty string"
            )
    tolerance = _number(root["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise TinyMessagePassingValidationError("absolute_tolerance must be positive")
    concepts = root["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or len(concepts) != len(set(concepts))
        or any(not isinstance(item, str) or not item for item in concepts)
    ):
        raise TinyMessagePassingValidationError(
            "concepts must contain unique non-empty strings"
        )
    operation = _object(
        root["operation"],
        {"kind", "edge_mode", "message", "aggregation", "update", "round"},
        "operation",
    )
    required = {
        "kind": "tiny-graph-message-passing",
        "edge_mode": "undirected-expanded-both-directions",
        "message": "source-feature-times-shared-weight",
        "aggregation": "sum",
        "update": "shared-affine-plus-relu",
        "round": "synchronous",
    }
    if operation != required:
        raise TinyMessagePassingValidationError("operation does not match NN21 V1")
    actual = execute_lab(root)
    if not isinstance(root["expected"], dict):
        raise TinyMessagePassingValidationError("expected: expected an object")
    _compare(actual, root["expected"], tolerance, "expected")
    return actual


def validate_corpus(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    load_json(root / "schema.json")
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise TinyMessagePassingValidationError("no lab documents found")
    seen: set[str] = set()
    for path in paths:
        document = load_json(path)
        if document.get("id") in seen:
            raise TinyMessagePassingValidationError(
                f"duplicate lab id: {document.get('id')}"
            )
        seen.add(str(document.get("id")))
        validate_document(document)
        print(f"validated {path}")
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_corpus(args.fixture_root)
    print(f"validated {count} NN21 tiny message-passing lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
