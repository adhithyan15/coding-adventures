#!/usr/bin/env python3
"""Validate and execute the deterministic NN12 attention QKV corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "attention-qkv-v1"


class AttentionQkvValidationError(ValueError):
    """Raised when an NN12 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AttentionQkvValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                AttentionQkvValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise AttentionQkvValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise AttentionQkvValidationError("top-level JSON value must be an object")
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise AttentionQkvValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise AttentionQkvValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise AttentionQkvValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise AttentionQkvValidationError(f"{context}: expected a finite number")
    return result


def _matrix(value: Any, rows: int, columns: int, context: str) -> list[list[float]]:
    if not isinstance(value, list) or len(value) != rows:
        raise AttentionQkvValidationError(f"{context}: expected {rows} rows")
    result: list[list[float]] = []
    for row_index, row in enumerate(value):
        if not isinstance(row, list) or len(row) != columns:
            raise AttentionQkvValidationError(
                f"{context}[{row_index}]: expected {columns} columns"
            )
        result.append(
            [
                _number(item, f"{context}[{row_index}][{column_index}]")
                for column_index, item in enumerate(row)
            ]
        )
    return result


def _matmul(left: list[list[float]], right: list[list[float]]) -> list[list[float]]:
    return [
        [
            sum(row[index] * right[index][column] for index in range(len(right)))
            for column in range(len(right[0]))
        ]
        for row in left
    ]


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    tokens = document["tokens"]
    if not isinstance(tokens, list) or len(tokens) != 3:
        raise AttentionQkvValidationError("tokens: expected three token objects")
    token_ids: list[str] = []
    embeddings: list[list[float]] = []
    for index, token_value in enumerate(tokens):
        token = _object(token_value, {"id", "embedding"}, f"tokens[{index}]")
        if not isinstance(token["id"], str) or not token["id"]:
            raise AttentionQkvValidationError(f"tokens[{index}].id: expected text")
        token_ids.append(token["id"])
        embeddings.append(
            _matrix([token["embedding"]], 1, 2, f"tokens[{index}].embedding")[0]
        )
    if len(set(token_ids)) != 3:
        raise AttentionQkvValidationError("tokens: ids must be unique")

    matrices = _object(document["matrices"], {"query", "key", "value"}, "matrices")
    query_matrix = _matrix(matrices["query"], 2, 2, "matrices.query")
    key_matrix = _matrix(matrices["key"], 2, 2, "matrices.key")
    value_matrix = _matrix(matrices["value"], 2, 2, "matrices.value")
    queries = _matmul(embeddings, query_matrix)
    keys = _matmul(embeddings, key_matrix)
    values = _matmul(embeddings, value_matrix)
    scale = math.sqrt(len(keys[0]))
    dot_products: list[dict[str, Any]] = []
    raw_score_matrix: list[list[float]] = []
    scaled_score_matrix: list[list[float]] = []
    for query_index, query in enumerate(queries):
        raw_row: list[float] = []
        scaled_row: list[float] = []
        for key_index, key in enumerate(keys):
            products = [query[index] * key[index] for index in range(len(key))]
            raw_score = sum(products)
            scaled_score = raw_score / scale
            raw_row.append(raw_score)
            scaled_row.append(scaled_score)
            dot_products.append(
                {
                    "query": token_ids[query_index],
                    "key": token_ids[key_index],
                    "products": products,
                    "raw_score": raw_score,
                    "scaled_score": scaled_score,
                }
            )
        raw_score_matrix.append(raw_row)
        scaled_score_matrix.append(scaled_row)
    return {
        "queries": queries,
        "keys": keys,
        "values": values,
        "dot_products": dot_products,
        "raw_score_matrix": raw_score_matrix,
        "scaled_score_matrix": scaled_score_matrix,
    }


def _compare(expected: Any, actual: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual:
            raise AttentionQkvValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if not math.isclose(
            float(expected), float(actual), rel_tol=0, abs_tol=tolerance
        ):
            raise AttentionQkvValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise AttentionQkvValidationError(f"{context}: length mismatch")
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise AttentionQkvValidationError(f"{context}: key mismatch")
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise AttentionQkvValidationError(
            f"{context}: expected {expected!r}, got {actual!r}"
        )


def validate_lab(document: dict[str, Any]) -> dict[str, Any]:
    _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "concepts",
            "operation",
            "tokens",
            "matrices",
            "expected",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise AttentionQkvValidationError("schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise AttentionQkvValidationError(f"{key}: expected text")
    tolerance = _number(document["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise AttentionQkvValidationError("absolute_tolerance: expected positive")
    operation = _object(
        document["operation"],
        {"kind", "vector_orientation", "score_scaling", "softmax_applied"},
        "operation",
    )
    if operation != {
        "kind": "three-token-qkv-dot-products",
        "vector_orientation": "row",
        "score_scaling": "inverse-square-root-key-dimension",
        "softmax_applied": False,
    }:
        raise AttentionQkvValidationError("operation: unsupported contract")
    expected = _object(
        document["expected"],
        {
            "queries",
            "keys",
            "values",
            "dot_products",
            "raw_score_matrix",
            "scaled_score_matrix",
        },
        "expected",
    )
    actual = execute_lab(document)
    _compare(expected, actual, tolerance, "expected")
    return actual


def validate_corpus(root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise AttentionQkvValidationError(f"no labs found under {root / 'labs'}")
    for path in paths:
        try:
            validate_lab(load_json(path))
        except AttentionQkvValidationError as error:
            raise AttentionQkvValidationError(f"{path}: {error}") from error
    return paths


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    paths = validate_corpus(args.root)
    print(f"validated {len(paths)} attention QKV labs from {args.root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
