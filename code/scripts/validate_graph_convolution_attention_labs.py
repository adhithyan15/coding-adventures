#!/usr/bin/env python3
"""Validate and execute deterministic NN22 GCN/GAT comparison labs."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "graph-convolution-attention-v1"
)


class GraphConvolutionAttentionValidationError(ValueError):
    """Raised when an NN22 document or trace is invalid."""


def _duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise GraphConvolutionAttentionValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_duplicates,
            parse_constant=lambda item: (_ for _ in ()).throw(
                GraphConvolutionAttentionValidationError(
                    f"non-finite JSON number: {item}"
                )
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise GraphConvolutionAttentionValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise GraphConvolutionAttentionValidationError(
            "top-level JSON must be an object"
        )
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise GraphConvolutionAttentionValidationError(f"{context}: expected object")
    missing, extra = keys - value.keys(), value.keys() - keys
    if missing or extra:
        raise GraphConvolutionAttentionValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(float(value))
    ):
        raise GraphConvolutionAttentionValidationError(
            f"{context}: expected finite number"
        )
    return float(value)


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    raw_features = document["features"]
    if not isinstance(raw_features, list) or len(raw_features) < 2:
        raise GraphConvolutionAttentionValidationError(
            "features: expected at least two values"
        )
    features = [
        _number(value, f"features[{index}]") for index, value in enumerate(raw_features)
    ]
    raw_neighborhoods = document["neighborhoods"]
    if not isinstance(raw_neighborhoods, list) or len(raw_neighborhoods) != len(
        features
    ):
        raise GraphConvolutionAttentionValidationError(
            "neighborhoods must align with features"
        )
    neighborhoods: list[list[int]] = []
    for target, raw_sources in enumerate(raw_neighborhoods):
        if (
            not isinstance(raw_sources, list)
            or not raw_sources
            or any(
                isinstance(source, bool) or not isinstance(source, int)
                for source in raw_sources
            )
        ):
            raise GraphConvolutionAttentionValidationError(
                f"neighborhoods[{target}]: invalid indices"
            )
        sources = list(raw_sources)
        if (
            len(sources) != len(set(sources))
            or target not in sources
            or any(source < 0 or source >= len(features) for source in sources)
        ):
            raise GraphConvolutionAttentionValidationError(
                f"neighborhoods[{target}]: must be unique valid indices including self"
            )
        neighborhoods.append(sources)
    for target, sources in enumerate(neighborhoods):
        if any(target not in neighborhoods[source] for source in sources):
            raise GraphConvolutionAttentionValidationError(
                "neighborhoods must be symmetric"
            )

    degrees = [len(sources) for sources in neighborhoods]
    gcn = []
    gat = []
    for target, sources in enumerate(neighborhoods):
        gcn_rows = []
        for source in sources:
            coefficient = 1 / math.sqrt(degrees[target] * degrees[source])
            gcn_rows.append(
                {
                    "source": source,
                    "source_feature": features[source],
                    "source_degree": degrees[source],
                    "target_degree": degrees[target],
                    "coefficient": coefficient,
                    "contribution": coefficient * features[source],
                }
            )
        gcn_preactivation = sum(row["contribution"] for row in gcn_rows)
        gcn.append(
            {
                "target": target,
                "rows": gcn_rows,
                "preactivation": gcn_preactivation,
                "output": max(0.0, gcn_preactivation),
            }
        )

        scores = [features[source] for source in sources]
        maximum = max(scores)
        exponentials = [math.exp(score - maximum) for score in scores]
        denominator = sum(exponentials)
        gat_rows = []
        for index, source in enumerate(sources):
            weight = exponentials[index] / denominator
            gat_rows.append(
                {
                    "source": source,
                    "source_feature": features[source],
                    "score": scores[index],
                    "shifted_score": scores[index] - maximum,
                    "exponential": exponentials[index],
                    "attention_weight": weight,
                    "contribution": weight * features[source],
                }
            )
        gat_preactivation = sum(row["contribution"] for row in gat_rows)
        gat.append(
            {
                "target": target,
                "maximum_score": maximum,
                "denominator": denominator,
                "rows": gat_rows,
                "preactivation": gat_preactivation,
                "output": max(0.0, gat_preactivation),
            }
        )
    return {
        "degrees": degrees,
        "gcn": gcn,
        "gat": gat,
        "gcn_outputs": [row["output"] for row in gcn],
        "gat_outputs": [row["output"] for row in gat],
    }


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise GraphConvolutionAttentionValidationError(f"{context}: mismatch")
    elif isinstance(expected, (int, float)):
        if abs(_number(actual, context) - float(expected)) > tolerance:
            raise GraphConvolutionAttentionValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str):
        if actual != expected:
            raise GraphConvolutionAttentionValidationError(f"{context}: mismatch")
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise GraphConvolutionAttentionValidationError(
                f"{context}: array length mismatch"
            )
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]")
    elif isinstance(expected, dict):
        value = _object(actual, set(expected), context)
        for key, right in expected.items():
            _compare(value[key], right, tolerance, f"{context}.{key}")
    else:
        raise GraphConvolutionAttentionValidationError(f"{context}: unsupported value")


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
            "features",
            "neighborhoods",
            "expected",
        },
        "document",
    )
    if root["schema_version"] != 1:
        raise GraphConvolutionAttentionValidationError("schema_version must be 1")
    for key in ("id", "title", "question"):
        if not isinstance(root[key], str) or not root[key].strip():
            raise GraphConvolutionAttentionValidationError(
                f"{key}: expected non-empty string"
            )
    tolerance = _number(root["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise GraphConvolutionAttentionValidationError(
            "absolute_tolerance must be positive"
        )
    concepts = root["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or len(concepts) != len(set(concepts))
    ):
        raise GraphConvolutionAttentionValidationError("concepts must be unique")
    operation = _object(
        root["operation"],
        {"kind", "gcn", "gat_score", "gat_normalization", "activation"},
        "operation",
    )
    required = {
        "kind": "graph-convolution-attention-comparison",
        "gcn": "symmetric-degree-normalized-sum",
        "gat_score": "source-feature",
        "gat_normalization": "stable-neighborhood-softmax",
        "activation": "relu",
    }
    if operation != required:
        raise GraphConvolutionAttentionValidationError(
            "operation does not match NN22 V1"
        )
    actual = execute_lab(root)
    if not isinstance(root["expected"], dict):
        raise GraphConvolutionAttentionValidationError("expected must be object")
    _compare(actual, root["expected"], tolerance, "expected")
    for row in actual["gat"]:
        if abs(sum(item["attention_weight"] for item in row["rows"]) - 1) > tolerance:
            raise GraphConvolutionAttentionValidationError(
                "attention row must sum to one"
            )
    return actual


def validate_corpus(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    load_json(root / "schema.json")
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise GraphConvolutionAttentionValidationError("no labs found")
    for path in paths:
        validate_document(load_json(path))
        print(f"validated {path}")
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_corpus(args.fixture_root)
    print(f"validated {count} NN22 graph convolution/attention lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
