#!/usr/bin/env python3
"""Validate and execute the deterministic NN13 attention softmax corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "attention-softmax-v1"
)


class AttentionSoftmaxValidationError(ValueError):
    """Raised when an NN13 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AttentionSoftmaxValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                AttentionSoftmaxValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise AttentionSoftmaxValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise AttentionSoftmaxValidationError("top-level JSON value must be an object")
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise AttentionSoftmaxValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise AttentionSoftmaxValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise AttentionSoftmaxValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise AttentionSoftmaxValidationError(f"{context}: expected a finite number")
    return result


def _matrix(value: Any, rows: int, columns: int, context: str) -> list[list[float]]:
    if not isinstance(value, list) or len(value) != rows:
        raise AttentionSoftmaxValidationError(f"{context}: expected {rows} rows")
    result: list[list[float]] = []
    for row_index, row in enumerate(value):
        if not isinstance(row, list) or len(row) != columns:
            raise AttentionSoftmaxValidationError(
                f"{context}[{row_index}]: expected {columns} columns"
            )
        result.append(
            [
                _number(item, f"{context}[{row_index}][{column_index}]")
                for column_index, item in enumerate(row)
            ]
        )
    return result


def _token_ids(value: Any) -> list[str]:
    if not isinstance(value, list) or len(value) != 3:
        raise AttentionSoftmaxValidationError("token_ids: expected three strings")
    if any(not isinstance(item, str) or not item for item in value):
        raise AttentionSoftmaxValidationError("token_ids: expected non-empty strings")
    if len(set(value)) != len(value):
        raise AttentionSoftmaxValidationError("token_ids: expected unique strings")
    return value


def _clean_zero(value: float) -> float:
    return 0.0 if abs(value) < 1e-15 else value


def _execute_mode(
    token_ids: list[str],
    scaled_scores: list[list[float]],
    values: list[list[float]],
    causal: bool,
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for query_index, score_row in enumerate(scaled_scores):
        allowed = [not causal or key_index <= query_index for key_index in range(3)]
        masked_scores: list[float | None] = [
            score if allowed[key_index] else None
            for key_index, score in enumerate(score_row)
        ]
        row_max = max(score for score in masked_scores if score is not None)
        shifted_scores: list[float | None] = [
            _clean_zero(score - row_max) if score is not None else None
            for score in masked_scores
        ]
        exponentials = [
            math.exp(score) if score is not None else 0.0 for score in shifted_scores
        ]
        denominator = sum(exponentials)
        weights = [_clean_zero(value / denominator) for value in exponentials]
        value_contributions = [
            [_clean_zero(weight * coordinate) for coordinate in values[key_index]]
            for key_index, weight in enumerate(weights)
        ]
        context = [
            _clean_zero(sum(row[coordinate] for row in value_contributions))
            for coordinate in range(2)
        ]
        rows.append(
            {
                "query": token_ids[query_index],
                "allowed": allowed,
                "scaled_scores": score_row,
                "masked_scores": masked_scores,
                "row_max": row_max,
                "shifted_scores": shifted_scores,
                "exponentials": exponentials,
                "denominator": denominator,
                "weights": weights,
                "value_contributions": value_contributions,
                "context": context,
            }
        )
    return rows


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    token_ids = _token_ids(document["token_ids"])
    scaled_scores = _matrix(
        document["scaled_score_matrix"], 3, 3, "scaled_score_matrix"
    )
    values = _matrix(document["values"], 3, 2, "values")
    return {
        "unmasked": _execute_mode(token_ids, scaled_scores, values, False),
        "causal": _execute_mode(token_ids, scaled_scores, values, True),
    }


def _compare(expected: Any, actual: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual:
            raise AttentionSoftmaxValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if not math.isclose(
            float(expected), float(actual), rel_tol=0, abs_tol=tolerance
        ):
            raise AttentionSoftmaxValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise AttentionSoftmaxValidationError(f"{context}: length mismatch")
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise AttentionSoftmaxValidationError(f"{context}: key mismatch")
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise AttentionSoftmaxValidationError(
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
            "token_ids",
            "scaled_score_matrix",
            "values",
            "expected",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise AttentionSoftmaxValidationError("schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise AttentionSoftmaxValidationError(f"{key}: expected text")
    concepts = document["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise AttentionSoftmaxValidationError("concepts: expected unique text values")
    tolerance = _number(document["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise AttentionSoftmaxValidationError("absolute_tolerance: expected positive")
    operation = _object(
        document["operation"],
        {"kind", "softmax", "causal_rule", "masked_score_encoding"},
        "operation",
    )
    if operation != {
        "kind": "three-token-attention-softmax",
        "softmax": "stable-row-wise",
        "causal_rule": "key-index-less-than-or-equal-query-index",
        "masked_score_encoding": "null",
    }:
        raise AttentionSoftmaxValidationError("operation: unsupported contract")
    expected = _object(document["expected"], {"unmasked", "causal"}, "expected")
    actual = execute_lab(document)
    _compare(expected, actual, tolerance, "expected")
    return actual


def validate_corpus(root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise AttentionSoftmaxValidationError(f"no labs found under {root / 'labs'}")
    for path in paths:
        try:
            validate_lab(load_json(path))
        except AttentionSoftmaxValidationError as error:
            raise AttentionSoftmaxValidationError(f"{path}: {error}") from error
    return paths


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    paths = validate_corpus(args.root)
    print(f"validated {len(paths)} attention softmax labs from {args.root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
