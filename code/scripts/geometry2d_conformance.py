"""Validate G2D00-G2D03 fixtures without consulting a package implementation.

The cases are a contract, not captured output. Recompute each answer from the
small published formulas, reject ambiguous transport, and never turn an
implementation's current behavior into its own expected value.
"""

from __future__ import annotations

import json
import math
import pathlib
import re
from itertools import pairwise
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[2]
CORPUS = ROOT / "code" / "specs" / "fixtures" / "geometry2d-v1" / "cases.json"
CASE_ID = re.compile(r"[a-z][a-z0-9-]{0,63}\Z")
RUNTIME_TOLERANCE = 1e-12
ORACLE_TOLERANCE = 1e-15


def _no_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """A duplicate JSON key must not choose an oracle by parser accident."""
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_json(source: str) -> Any:
    return json.loads(source, object_pairs_hook=_no_duplicate_keys)


def _fields(value: Any, required: set[str], where: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != required:
        raise ValueError(f"{where} fields must be exactly {sorted(required)}")
    return value


def _number(value: Any, where: str) -> float:
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(value)
        or abs(value) > 1_000_000
    ):
        raise ValueError(f"{where} must be a bounded finite number")
    return float(value)


def _tuple(value: Any, size: int, where: str) -> tuple[float, ...]:
    if not isinstance(value, list) or len(value) != size:
        raise ValueError(f"{where} must contain exactly {size} finite numbers")
    return tuple(_number(item, where) for item in value)


def _point(value: Any, where: str) -> tuple[float, float]:
    x, y = _tuple(value, 2, where)
    return (x, y)


def _expected(value: Any, answer: tuple[float, ...], where: str) -> None:
    published = _tuple(value, len(answer), where)
    if any(
        not math.isclose(got, want, rel_tol=0.0, abs_tol=ORACLE_TOLERANCE)
        for got, want in zip(published, answer, strict=True)
    ):
        raise ValueError(f"{where} differs from independently computed expected result")


def _affine_multiply(
    first: tuple[float, ...], second: tuple[float, ...]
) -> tuple[float, ...]:
    a, b, c, d, e, f = first
    g, h, i, j, k, l = second
    return (
        a * g + c * h,
        b * g + d * h,
        a * i + c * j,
        b * i + d * j,
        a * k + c * l + e,
        b * k + d * l + f,
    )


def _affine_apply(
    matrix: tuple[float, ...], point: tuple[float, ...], *, vector: bool = False
) -> tuple[float, float]:
    a, b, c, d, e, f = matrix
    x, y = point
    return (a * x + c * y + (0 if vector else e), b * x + d * y + (0 if vector else f))


def _affine_inverse(matrix: tuple[float, ...]) -> tuple[float, ...] | None:
    a, b, c, d, e, f = matrix
    determinant = a * d - b * c
    if abs(determinant) < 1e-12:
        return None
    return (
        d / determinant,
        -b / determinant,
        -c / determinant,
        a / determinant,
        (c * f - d * e) / determinant,
        (b * e - a * f) / determinant,
    )


def _lerp(
    first: tuple[float, float], second: tuple[float, float], t: float
) -> tuple[float, float]:
    return (
        first[0] + (second[0] - first[0]) * t,
        first[1] + (second[1] - first[1]) * t,
    )


def _bezier_evaluate(
    points: tuple[tuple[float, float], ...], t: float
) -> tuple[float, float]:
    """Use the Bernstein basis, independently of the split construction."""
    degree = len(points) - 1
    weights = [
        math.comb(degree, index) * (1 - t) ** (degree - index) * t**index
        for index in range(degree + 1)
    ]
    return (
        sum(weight * point[0] for weight, point in zip(weights, points, strict=True)),
        sum(weight * point[1] for weight, point in zip(weights, points, strict=True)),
    )


def _bezier_derivative(
    points: tuple[tuple[float, float], ...], t: float
) -> tuple[float, float]:
    degree = len(points) - 1
    differences = tuple(
        (degree * (right[0] - left[0]), degree * (right[1] - left[1]))
        for left, right in pairwise(points)
    )
    return _bezier_evaluate(differences, t)


def _bezier_split(
    points: tuple[tuple[float, float], ...], t: float
) -> tuple[tuple[tuple[float, float], ...], ...]:
    rows = [points]
    while len(rows[-1]) > 1:
        prior = rows[-1]
        rows.append(tuple(_lerp(left, right, t) for left, right in pairwise(prior)))
    left = tuple(row[0] for row in rows)
    right = tuple(row[-1] for row in reversed(rows))
    return (left, right)


def _bezier_bounds(points: tuple[tuple[float, float], ...]) -> tuple[float, ...]:
    candidates = {0.0, 1.0}
    for axis in (0, 1):
        coordinates = [point[axis] for point in points]
        if len(points) == 3:
            a, b, c = coordinates
            denominator = a - 2 * b + c
            roots = [(a - b) / denominator] if denominator != 0 else []
        else:
            a, b, c, d = coordinates
            alpha = b - a
            beta = 2 * (a - 2 * b + c)
            gamma = -a + 3 * b - 3 * c + d
            if abs(gamma) < 1e-12:
                roots = [-alpha / beta] if abs(beta) >= 1e-12 else []
            else:
                discriminant = beta * beta - 4 * gamma * alpha
                roots = (
                    []
                    if discriminant < 0
                    else [
                        (-beta - math.sqrt(discriminant)) / (2 * gamma),
                        (-beta + math.sqrt(discriminant)) / (2 * gamma),
                    ]
                )
        candidates.update(root for root in roots if 0.0 <= root <= 1.0)
    evaluated = [_bezier_evaluate(points, t) for t in candidates]
    min_x = min(point[0] for point in evaluated)
    min_y = min(point[1] for point in evaluated)
    max_x = max(point[0] for point in evaluated)
    max_y = max(point[1] for point in evaluated)
    return (min_x, min_y, max_x - min_x, max_y - min_y)


def _validate_case(case: Any) -> str:
    if not isinstance(case, dict):
        raise ValueError("case fields must form an object")  # noqa: TRY004 - stable corpus error
    case_id = case.get("id")
    if not isinstance(case_id, str) or CASE_ID.fullmatch(case_id) is None:
        raise ValueError("case id must be canonical lowercase hyphenated text")
    operation = case.get("operation")
    if operation == "point-normalize":
        _fields(case, {"id", "operation", "point", "expected", "comparison"}, case_id)
        x, y = _tuple(case["point"], 2, f"{case_id}.point")
        magnitude = math.hypot(x, y)
        if magnitude < 1e-12:
            answer = (0.0, 0.0)
        else:
            answer = (x / magnitude, y / magnitude)
        comparison = "exact" if answer == (0.0, 0.0) else "absolute"
        if case["comparison"] != comparison:
            raise ValueError(
                f"{case_id}.comparison must pin {comparison} output equality"
            )
        if comparison == "exact":
            if _tuple(case["expected"], 2, f"{case_id}.expected") != answer:
                raise ValueError(f"{case_id}.expected must equal the exact origin")
        else:
            _expected(case["expected"], answer, f"{case_id}.expected")
    elif operation == "svg-arc-degenerate":
        _fields(
            case,
            {
                "id",
                "operation",
                "from",
                "to",
                "rx",
                "ry",
                "sample_t",
                "expected_point",
                "expected_bounds",
            },
            case_id,
        )
        x0, y0 = _tuple(case["from"], 2, f"{case_id}.from")
        x1, y1 = _tuple(case["to"], 2, f"{case_id}.to")
        rx = _number(case["rx"], f"{case_id}.rx")
        ry = _number(case["ry"], f"{case_id}.ry")
        t = _number(case["sample_t"], f"{case_id}.sample_t")
        if not 0.0 <= t <= 1.0:
            raise ValueError(f"{case_id}.sample_t must be in [0,1]")
        if not (
            (x1 - x0) ** 2 + (y1 - y0) ** 2 < 1e-20
            or abs(rx) < 1e-10
            or abs(ry) < 1e-10
        ):
            raise ValueError(f"{case_id} is not degenerate under G2D03")
        _expected(
            case["expected_point"],
            (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t),
            f"{case_id}.expected_point",
        )
        _expected(
            case["expected_bounds"],
            (min(x0, x1), min(y0, y1), abs(x1 - x0), abs(y1 - y0)),
            f"{case_id}.expected_bounds",
        )
    elif operation == "affine-compose":
        _fields(
            case,
            {
                "id",
                "operation",
                "first",
                "second",
                "point",
                "expected_matrix",
                "expected_point",
            },
            case_id,
        )
        first = _tuple(case["first"], 6, f"{case_id}.first")
        second = _tuple(case["second"], 6, f"{case_id}.second")
        point = _tuple(case["point"], 2, f"{case_id}.point")
        result = _affine_multiply(first, second)
        _expected(case["expected_matrix"], result, f"{case_id}.expected_matrix")
        _expected(
            case["expected_point"],
            _affine_apply(result, point),
            f"{case_id}.expected_point",
        )
    elif operation == "affine-invert":
        _fields(case, {"id", "operation", "matrix", "expected_inverse"}, case_id)
        matrix = _tuple(case["matrix"], 6, f"{case_id}.matrix")
        inverse = _affine_inverse(matrix)
        if inverse is None:
            if case["expected_inverse"] is not None:
                raise ValueError(f"{case_id}.expected_inverse must be absent")
        else:
            _expected(case["expected_inverse"], inverse, f"{case_id}.expected_inverse")
            identity = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
            for product in (
                _affine_multiply(matrix, inverse),
                _affine_multiply(inverse, matrix),
            ):
                if any(
                    not math.isclose(got, want, rel_tol=0.0, abs_tol=RUNTIME_TOLERANCE)
                    for got, want in zip(product, identity, strict=True)
                ):
                    raise ValueError(f"{case_id}.expected_inverse does not compose")
    elif operation == "affine-vector":
        _fields(case, {"id", "operation", "matrix", "vector", "expected"}, case_id)
        matrix = _tuple(case["matrix"], 6, f"{case_id}.matrix")
        vector = _tuple(case["vector"], 2, f"{case_id}.vector")
        _expected(
            case["expected"],
            _affine_apply(matrix, vector, vector=True),
            f"{case_id}.expected",
        )
    elif operation in ("bezier-quadratic", "bezier-cubic"):
        _fields(
            case,
            {
                "id",
                "operation",
                "control_points",
                "t",
                "expected_point",
                "expected_derivative",
                "expected_split",
                "expected_bounds",
            },
            case_id,
        )
        size = 3 if operation == "bezier-quadratic" else 4
        raw_points = case["control_points"]
        if not isinstance(raw_points, list) or len(raw_points) != size:
            raise ValueError(f"{case_id}.control_points needs exactly {size} points")
        points = tuple(
            _point(point, f"{case_id}.control_points") for point in raw_points
        )
        t = _number(case["t"], f"{case_id}.t")
        if not 0.0 <= t <= 1.0:
            raise ValueError(f"{case_id}.t must be in [0,1]")
        _expected(
            case["expected_point"],
            _bezier_evaluate(points, t),
            f"{case_id}.expected_point",
        )
        _expected(
            case["expected_derivative"],
            _bezier_derivative(points, t),
            f"{case_id}.expected_derivative",
        )
        split = case["expected_split"]
        if not isinstance(split, list) or len(split) != 2:
            raise ValueError(f"{case_id}.expected_split needs two control polygons")
        for side_index, expected_side in enumerate(_bezier_split(points, t)):
            published_side = split[side_index]
            if not isinstance(published_side, list) or len(published_side) != size:
                raise ValueError(f"{case_id}.expected_split side shape is invalid")
            for point_index, expected_point in enumerate(expected_side):
                _expected(
                    published_side[point_index],
                    expected_point,
                    f"{case_id}.expected_split[{side_index}][{point_index}]",
                )
        _expected(
            case["expected_bounds"],
            _bezier_bounds(points),
            f"{case_id}.expected_bounds",
        )
    else:
        raise ValueError(f"{case_id} has unsupported operation")
    return case_id


def validate_document(document: Any) -> int:
    root = _fields(document, {"version", "absolute_tolerance", "cases"}, "corpus")
    if type(root["version"]) is not int or root["version"] != 1:
        raise ValueError("unsupported corpus version")
    if (
        type(root["absolute_tolerance"]) not in (int, float)
        or root["absolute_tolerance"] != RUNTIME_TOLERANCE
    ):
        raise ValueError("absolute tolerance must remain pinned")
    cases = root["cases"]
    if not isinstance(cases, list) or not 1 <= len(cases) <= 64:
        raise ValueError("cases must be a bounded nonempty list")
    seen: set[str] = set()
    for case in cases:
        case_id = _validate_case(case)
        if case_id in seen:
            raise ValueError(f"duplicate case id: {case_id}")
        seen.add(case_id)
    return len(cases)


def main() -> None:
    count = validate_document(parse_json(CORPUS.read_text(encoding="utf-8")))
    print(f"validated {count} neutral G2D00-G2D03 cases")


if __name__ == "__main__":
    main()
