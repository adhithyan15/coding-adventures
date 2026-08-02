#!/usr/bin/env python3
"""Validate and execute the deterministic NN14 multi-head attention corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "multi-head-attention-v1"
)


class MultiHeadAttentionValidationError(ValueError):
    """Raised when an NN14 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise MultiHeadAttentionValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                MultiHeadAttentionValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise MultiHeadAttentionValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise MultiHeadAttentionValidationError(
            "top-level JSON value must be an object"
        )
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise MultiHeadAttentionValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise MultiHeadAttentionValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise MultiHeadAttentionValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise MultiHeadAttentionValidationError(f"{context}: expected a finite number")
    return result


def _vector(value: Any, length: int, context: str) -> list[float]:
    if not isinstance(value, list) or len(value) != length:
        raise MultiHeadAttentionValidationError(f"{context}: expected {length} numbers")
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _matrix(value: Any, rows: int, columns: int, context: str) -> list[list[float]]:
    if not isinstance(value, list) or len(value) != rows:
        raise MultiHeadAttentionValidationError(f"{context}: expected {rows} rows")
    return [
        _vector(row, columns, f"{context}[{index}]") for index, row in enumerate(value)
    ]


def _text_list(value: Any, length: int, context: str) -> list[str]:
    if (
        not isinstance(value, list)
        or len(value) != length
        or any(not isinstance(item, str) or not item for item in value)
    ):
        raise MultiHeadAttentionValidationError(
            f"{context}: expected {length} non-empty strings"
        )
    if len(set(value)) != len(value):
        raise MultiHeadAttentionValidationError(f"{context}: expected unique strings")
    return value


def _clean_zero(value: float) -> float:
    return 0.0 if abs(value) < 1e-15 else value


def _dot_products(vector: list[float], projection: list[float]) -> list[float]:
    return [_clean_zero(value * weight) for value, weight in zip(vector, projection)]


def _execute_head(
    embeddings: list[list[float]],
    query_index: int,
    head: dict[str, Any],
) -> dict[str, Any]:
    query_projection = head["query_projection"]
    key_projection = head["key_projection"]
    value_projection = head["value_projection"]
    query_products = _dot_products(embeddings[query_index], query_projection)
    query = _clean_zero(sum(query_products))
    key_products = [
        _dot_products(embedding, key_projection) for embedding in embeddings
    ]
    keys = [_clean_zero(sum(products)) for products in key_products]
    value_products = [
        _dot_products(embedding, value_projection) for embedding in embeddings
    ]
    values = [_clean_zero(sum(products)) for products in value_products]
    scale_divisor = 1.0
    scaled_scores = [_clean_zero(query * key / scale_divisor) for key in keys]
    allowed = [key_index <= query_index for key_index in range(3)]
    masked_scores: list[float | None] = [
        score if allowed[key_index] else None
        for key_index, score in enumerate(scaled_scores)
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
        _clean_zero(weight * value) for weight, value in zip(weights, values)
    ]
    return {
        "id": head["id"],
        "query_products": query_products,
        "query": query,
        "key_products": key_products,
        "keys": keys,
        "value_products": value_products,
        "values": values,
        "scale_divisor": scale_divisor,
        "scaled_scores": scaled_scores,
        "allowed": allowed,
        "masked_scores": masked_scores,
        "row_max": row_max,
        "shifted_scores": shifted_scores,
        "exponentials": exponentials,
        "denominator": denominator,
        "weights": weights,
        "value_contributions": value_contributions,
        "context": _clean_zero(sum(value_contributions)),
    }


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    embeddings = _matrix(document["embeddings"], 3, 2, "embeddings")
    output_projection = _matrix(
        document["output_projection"], 2, 2, "output_projection"
    )
    normalization = document["normalization"]
    epsilon = _number(normalization["epsilon"], "normalization.epsilon")
    gamma = _vector(normalization["gamma"], 2, "normalization.gamma")
    beta = _vector(normalization["beta"], 2, "normalization.beta")
    rows: list[dict[str, Any]] = []

    for query_index, embedding in enumerate(embeddings):
        head_traces = [
            _execute_head(embeddings, query_index, head) for head in document["heads"]
        ]
        concatenated = [head["context"] for head in head_traces]
        output_projection_products = [
            [
                _clean_zero(
                    concatenated[head_index]
                    * output_projection[head_index][output_index]
                )
                for head_index in range(2)
            ]
            for output_index in range(2)
        ]
        projected_attention = [
            _clean_zero(sum(products)) for products in output_projection_products
        ]
        residual_sum = [
            _clean_zero(embedding[index] + projected_attention[index])
            for index in range(2)
        ]
        mean = sum(residual_sum) / 2
        centered = [_clean_zero(value - mean) for value in residual_sum]
        squared_deviations = [value * value for value in centered]
        variance = sum(squared_deviations) / 2
        denominator = math.sqrt(variance + epsilon)
        normalized = [_clean_zero(value / denominator) for value in centered]
        affine_products = [
            _clean_zero(value * gamma[index]) for index, value in enumerate(normalized)
        ]
        output = [
            _clean_zero(value + beta[index])
            for index, value in enumerate(affine_products)
        ]
        rows.append(
            {
                "token": document["token_ids"][query_index],
                "input": embedding,
                "heads": head_traces,
                "concatenated": concatenated,
                "output_projection_products": output_projection_products,
                "projected_attention": projected_attention,
                "residual_sum": residual_sum,
                "layer_norm": {
                    "mean": mean,
                    "centered": centered,
                    "squared_deviations": squared_deviations,
                    "variance": variance,
                    "denominator": denominator,
                    "normalized": normalized,
                    "affine_products": affine_products,
                    "output": output,
                },
            }
        )
    return {"rows": rows}


def _compare(expected: Any, actual: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual:
            raise MultiHeadAttentionValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if not math.isclose(
            float(expected), float(actual), rel_tol=0, abs_tol=tolerance
        ):
            raise MultiHeadAttentionValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise MultiHeadAttentionValidationError(f"{context}: length mismatch")
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise MultiHeadAttentionValidationError(f"{context}: key mismatch")
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise MultiHeadAttentionValidationError(
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
            "embeddings",
            "heads",
            "output_projection",
            "normalization",
            "expected",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise MultiHeadAttentionValidationError("schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise MultiHeadAttentionValidationError(f"{key}: expected text")
    concepts = document["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise MultiHeadAttentionValidationError(
            "concepts: expected unique non-empty strings"
        )
    tolerance = _number(document["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise MultiHeadAttentionValidationError(
            "absolute_tolerance: expected a positive number"
        )

    operation = _object(
        document["operation"],
        {
            "kind",
            "head_count",
            "model_width",
            "head_width",
            "softmax",
            "causal_rule",
            "masked_score_encoding",
            "block_order",
            "layer_norm_variance",
        },
        "operation",
    )
    required_operation = {
        "kind": "two-head-causal-attention-add-norm",
        "head_count": 2,
        "model_width": 2,
        "head_width": 1,
        "softmax": "stable-row-wise",
        "causal_rule": "key-index-less-than-or-equal-query-index",
        "masked_score_encoding": "null",
        "block_order": "attention-output-projection-add-residual-layer-norm",
        "layer_norm_variance": "population",
    }
    if operation != required_operation:
        raise MultiHeadAttentionValidationError("operation: unsupported NN14 contract")

    _text_list(document["token_ids"], 3, "token_ids")
    _matrix(document["embeddings"], 3, 2, "embeddings")
    heads = document["heads"]
    if not isinstance(heads, list) or len(heads) != 2:
        raise MultiHeadAttentionValidationError("heads: expected two heads")
    head_ids: list[str] = []
    for index, head_value in enumerate(heads):
        head = _object(
            head_value,
            {"id", "query_projection", "key_projection", "value_projection"},
            f"heads[{index}]",
        )
        if not isinstance(head["id"], str) or not head["id"]:
            raise MultiHeadAttentionValidationError(f"heads[{index}].id: expected text")
        head_ids.append(head["id"])
        for projection in ("query_projection", "key_projection", "value_projection"):
            head[projection] = _vector(
                head[projection], 2, f"heads[{index}].{projection}"
            )
    if len(set(head_ids)) != 2:
        raise MultiHeadAttentionValidationError("heads: expected unique IDs")
    _matrix(document["output_projection"], 2, 2, "output_projection")
    normalization = _object(
        document["normalization"],
        {"epsilon", "gamma", "beta"},
        "normalization",
    )
    if _number(normalization["epsilon"], "normalization.epsilon") <= 0:
        raise MultiHeadAttentionValidationError(
            "normalization.epsilon: expected a positive number"
        )
    _vector(normalization["gamma"], 2, "normalization.gamma")
    _vector(normalization["beta"], 2, "normalization.beta")
    expected = _object(document["expected"], {"rows"}, "expected")
    actual = execute_lab(document)
    _compare(expected, actual, tolerance, "expected")
    return actual


def validate_fixture_root(root: Path) -> int:
    labs = sorted((root / "labs").glob("*.json"))
    if not labs:
        raise MultiHeadAttentionValidationError(f"{root}: no lab documents found")
    for path in labs:
        validate_lab(load_json(path))
        print(f"validated {path.relative_to(REPO_ROOT)}")
    return len(labs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_fixture_root(args.root.resolve())
    print(f"validated {count} NN14 multi-head attention lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
