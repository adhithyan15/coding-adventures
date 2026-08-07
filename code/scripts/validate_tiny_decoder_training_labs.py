#!/usr/bin/env python3
"""Validate and execute the deterministic NN15 tiny decoder training corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "tiny-decoder-training-v1"
)


class TinyDecoderTrainingValidationError(ValueError):
    """Raised when an NN15 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise TinyDecoderTrainingValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                TinyDecoderTrainingValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise TinyDecoderTrainingValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise TinyDecoderTrainingValidationError(
            "top-level JSON value must be an object"
        )
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise TinyDecoderTrainingValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise TinyDecoderTrainingValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TinyDecoderTrainingValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise TinyDecoderTrainingValidationError(f"{context}: expected a finite number")
    return result


def _vector(value: Any, length: int, context: str) -> list[float]:
    if not isinstance(value, list) or len(value) != length:
        raise TinyDecoderTrainingValidationError(
            f"{context}: expected {length} numbers"
        )
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _matrix(value: Any, rows: int, columns: int, context: str) -> list[list[float]]:
    if not isinstance(value, list) or len(value) != rows:
        raise TinyDecoderTrainingValidationError(f"{context}: expected {rows} rows")
    return [
        _vector(row, columns, f"{context}[{index}]") for index, row in enumerate(value)
    ]


def _text_list(
    value: Any,
    length: int,
    context: str,
    *,
    unique: bool = False,
) -> list[str]:
    if (
        not isinstance(value, list)
        or len(value) != length
        or any(not isinstance(item, str) or not item for item in value)
    ):
        raise TinyDecoderTrainingValidationError(
            f"{context}: expected {length} non-empty strings"
        )
    if unique and len(set(value)) != length:
        raise TinyDecoderTrainingValidationError(f"{context}: expected unique strings")
    return value


def _clean_zero(value: float) -> float:
    return 0.0 if abs(value) < 1e-15 else value


def _forward_row(
    state: list[float],
    target_index: int,
    unembedding: list[list[float]],
    bias: list[float],
) -> dict[str, Any]:
    logit_products = [
        [
            _clean_zero(state[dimension] * unembedding[dimension][vocab_index])
            for dimension in range(2)
        ]
        for vocab_index in range(3)
    ]
    logits = [
        _clean_zero(sum(products) + bias[vocab_index])
        for vocab_index, products in enumerate(logit_products)
    ]
    row_max = max(logits)
    shifted_logits = [_clean_zero(logit - row_max) for logit in logits]
    exponentials = [math.exp(logit) for logit in shifted_logits]
    denominator = sum(exponentials)
    probabilities = [value / denominator for value in exponentials]
    target_probability = probabilities[target_index]
    return {
        "logit_products": logit_products,
        "logits": logits,
        "row_max": row_max,
        "shifted_logits": shifted_logits,
        "exponentials": exponentials,
        "denominator": denominator,
        "probabilities": probabilities,
        "target_probability": target_probability,
        "loss": -math.log(target_probability),
    }


def _mean_loss(
    states: list[list[float]],
    target_indices: list[int],
    unembedding: list[list[float]],
    bias: list[float],
) -> float:
    return sum(
        _forward_row(state, target_index, unembedding, bias)["loss"]
        for state, target_index in zip(states, target_indices)
    ) / len(states)


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    vocabulary = _text_list(document["vocabulary"], 3, "vocabulary", unique=True)
    states = _matrix(document["decoder_states"], 2, 2, "decoder_states")
    unembedding = _matrix(document["unembedding"], 2, 3, "unembedding")
    bias = _vector(document["bias"], 3, "bias")
    target_tokens = _text_list(document["target_tokens"], 2, "target_tokens")
    target_indices = [vocabulary.index(token) for token in target_tokens]
    learning_rate = _number(
        document["optimizer"]["learning_rate"], "optimizer.learning_rate"
    )

    rows: list[dict[str, Any]] = []
    unembedding_gradient = [[0.0] * 3 for _ in range(2)]
    bias_gradient = [0.0] * 3
    for position, (state, target_index) in enumerate(zip(states, target_indices)):
        forward = _forward_row(state, target_index, unembedding, bias)
        probabilities = forward["probabilities"]
        logit_gradients = [
            (probability - (1.0 if vocab_index == target_index else 0.0)) / 2
            for vocab_index, probability in enumerate(probabilities)
        ]
        contribution = [
            [
                _clean_zero(state[dimension] * logit_gradients[vocab_index])
                for vocab_index in range(3)
            ]
            for dimension in range(2)
        ]
        for dimension in range(2):
            for vocab_index in range(3):
                unembedding_gradient[dimension][vocab_index] += contribution[dimension][
                    vocab_index
                ]
        for vocab_index in range(3):
            bias_gradient[vocab_index] += logit_gradients[vocab_index]
        state_gradient = [
            _clean_zero(
                sum(
                    logit_gradients[vocab_index] * unembedding[dimension][vocab_index]
                    for vocab_index in range(3)
                )
            )
            for dimension in range(2)
        ]
        rows.append(
            {
                "position": position,
                "input_token": document["input_tokens"][position],
                "target_token": target_tokens[position],
                "causal_prefix": document["causal_prefixes"][position],
                "decoder_state": state,
                **forward,
                "logit_gradients": logit_gradients,
                "unembedding_gradient_contribution": contribution,
                "bias_gradient_contribution": logit_gradients,
                "state_gradient": state_gradient,
            }
        )

    mean_loss = sum(row["loss"] for row in rows) / 2
    gradient_epsilon = 1e-6
    numerical_unembedding_gradient = [[0.0] * 3 for _ in range(2)]
    for dimension in range(2):
        for vocabulary_index in range(3):
            plus = [row[:] for row in unembedding]
            minus = [row[:] for row in unembedding]
            plus[dimension][vocabulary_index] += gradient_epsilon
            minus[dimension][vocabulary_index] -= gradient_epsilon
            numerical_unembedding_gradient[dimension][vocabulary_index] = (
                _mean_loss(states, target_indices, plus, bias)
                - _mean_loss(states, target_indices, minus, bias)
            ) / (2 * gradient_epsilon)
    numerical_bias_gradient = []
    for vocabulary_index in range(3):
        plus_bias = bias[:]
        minus_bias = bias[:]
        plus_bias[vocabulary_index] += gradient_epsilon
        minus_bias[vocabulary_index] -= gradient_epsilon
        numerical_bias_gradient.append(
            (
                _mean_loss(states, target_indices, unembedding, plus_bias)
                - _mean_loss(states, target_indices, unembedding, minus_bias)
            )
            / (2 * gradient_epsilon)
        )
    gradient_errors = [
        abs(
            numerical_unembedding_gradient[dimension][vocabulary_index]
            - unembedding_gradient[dimension][vocabulary_index]
        )
        for dimension in range(2)
        for vocabulary_index in range(3)
    ] + [
        abs(numerical_bias_gradient[index] - bias_gradient[index]) for index in range(3)
    ]
    updated_unembedding = [
        [
            unembedding[dimension][vocab_index]
            - learning_rate * unembedding_gradient[dimension][vocab_index]
            for vocab_index in range(3)
        ]
        for dimension in range(2)
    ]
    updated_bias = [
        bias[index] - learning_rate * bias_gradient[index] for index in range(3)
    ]
    post_update_rows = []
    for position, (state, target_index) in enumerate(zip(states, target_indices)):
        forward = _forward_row(state, target_index, updated_unembedding, updated_bias)
        post_update_rows.append(
            {
                "position": position,
                "logits": forward["logits"],
                "probabilities": forward["probabilities"],
                "target_probability": forward["target_probability"],
                "loss": forward["loss"],
            }
        )

    return {
        "rows": rows,
        "mean_loss": mean_loss,
        "unembedding_gradient": unembedding_gradient,
        "bias_gradient": bias_gradient,
        "gradient_check": {
            "epsilon": gradient_epsilon,
            "numerical_unembedding_gradient": numerical_unembedding_gradient,
            "numerical_bias_gradient": numerical_bias_gradient,
            "max_absolute_error": max(gradient_errors),
        },
        "updated_unembedding": updated_unembedding,
        "updated_bias": updated_bias,
        "post_update_rows": post_update_rows,
        "post_update_mean_loss": sum(row["loss"] for row in post_update_rows) / 2,
    }


def _compare(expected: Any, actual: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual:
            raise TinyDecoderTrainingValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if not math.isclose(
            float(expected), float(actual), rel_tol=0, abs_tol=tolerance
        ):
            raise TinyDecoderTrainingValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise TinyDecoderTrainingValidationError(f"{context}: length mismatch")
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise TinyDecoderTrainingValidationError(f"{context}: key mismatch")
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise TinyDecoderTrainingValidationError(
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
            "vocabulary",
            "sequence",
            "input_tokens",
            "target_tokens",
            "causal_prefixes",
            "decoder_states",
            "unembedding",
            "bias",
            "optimizer",
            "expected",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise TinyDecoderTrainingValidationError("schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise TinyDecoderTrainingValidationError(f"{key}: expected text")
    concepts = document["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise TinyDecoderTrainingValidationError(
            "concepts: expected unique non-empty strings"
        )
    tolerance = _number(document["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise TinyDecoderTrainingValidationError(
            "absolute_tolerance: expected a positive number"
        )

    operation = _object(
        document["operation"],
        {
            "kind",
            "position_count",
            "model_width",
            "vocabulary_size",
            "state_source",
            "objective",
            "softmax",
            "reduction",
            "trainable_parameters",
            "update",
        },
        "operation",
    )
    required_operation = {
        "kind": "saved-causal-decoder-state-next-token-step",
        "position_count": 2,
        "model_width": 2,
        "vocabulary_size": 3,
        "state_source": "frozen-causal-decoder-block-output",
        "objective": "next-token-cross-entropy",
        "softmax": "stable-row-wise",
        "reduction": "mean-over-positions",
        "trainable_parameters": ["unembedding", "bias"],
        "update": "sgd",
    }
    if operation != required_operation:
        raise TinyDecoderTrainingValidationError("operation: unsupported NN15 contract")

    vocabulary = _text_list(document["vocabulary"], 3, "vocabulary", unique=True)
    sequence = _text_list(document["sequence"], 3, "sequence")
    input_tokens = _text_list(document["input_tokens"], 2, "input_tokens")
    target_tokens = _text_list(document["target_tokens"], 2, "target_tokens")
    if any(token not in vocabulary for token in sequence):
        raise TinyDecoderTrainingValidationError("sequence: token outside vocabulary")
    if input_tokens != sequence[:-1] or target_tokens != sequence[1:]:
        raise TinyDecoderTrainingValidationError(
            "sequence shift: inputs and targets must be adjacent slices"
        )
    prefixes = document["causal_prefixes"]
    if prefixes != [sequence[:1], sequence[:2]]:
        raise TinyDecoderTrainingValidationError(
            "causal_prefixes: expected growing sequence prefixes"
        )
    _matrix(document["decoder_states"], 2, 2, "decoder_states")
    _matrix(document["unembedding"], 2, 3, "unembedding")
    _vector(document["bias"], 3, "bias")
    optimizer = _object(document["optimizer"], {"learning_rate"}, "optimizer")
    if _number(optimizer["learning_rate"], "optimizer.learning_rate") <= 0:
        raise TinyDecoderTrainingValidationError(
            "optimizer.learning_rate: expected a positive number"
        )
    expected = _object(
        document["expected"],
        {
            "rows",
            "mean_loss",
            "unembedding_gradient",
            "bias_gradient",
            "gradient_check",
            "updated_unembedding",
            "updated_bias",
            "post_update_rows",
            "post_update_mean_loss",
        },
        "expected",
    )
    actual = execute_lab(document)
    _compare(expected, actual, tolerance, "expected")
    if actual["post_update_mean_loss"] >= actual["mean_loss"]:
        raise TinyDecoderTrainingValidationError(
            "expected.post_update_mean_loss: canonical update must lower loss"
        )
    return actual


def validate_fixture_root(root: Path) -> int:
    labs = sorted((root / "labs").glob("*.json"))
    if not labs:
        raise TinyDecoderTrainingValidationError(f"{root}: no lab documents found")
    for path in labs:
        validate_lab(load_json(path))
        print(f"validated {path.relative_to(REPO_ROOT)}")
    return len(labs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_fixture_root(args.root.resolve())
    print(f"validated {count} NN15 tiny decoder training lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
