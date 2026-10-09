"""Recompute G2D02 flattening witnesses without importing a Bezier package.

The fixtures describe the *contract*, not captured implementation output.
de Casteljau interpolation checks the witness while a separate finite-segment
projection checks distance. Reject unknown shape, duplicates, and nonfinite
transport before a native adapter can mistake them for a passing case.
"""

from __future__ import annotations

import json
import math
import pathlib
import re
from itertools import pairwise
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[2]
CORPUS = ROOT / "code" / "specs" / "fixtures" / "bezier2d-flattening-v1" / "cases.json"
CASE_ID = re.compile(r"[a-z][a-z0-9-]{0,63}\Z")
ORACLE_TOLERANCE = 1e-12


def _no_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value: str) -> None:
    raise ValueError(f"nonfinite JSON constant: {value}")


def parse_json(source: str) -> Any:
    return json.loads(
        source, object_pairs_hook=_no_duplicate_keys, parse_constant=_reject_constant
    )


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


def _point(value: Any, where: str) -> tuple[float, float]:
    if not isinstance(value, list) or len(value) != 2:
        raise ValueError(f"{where} must be a point of exactly two coordinates")
    return (_number(value[0], where), _number(value[1], where))


def _distance_to_segment(
    point: tuple[float, float], start: tuple[float, float], end: tuple[float, float]
) -> float:
    dx, dy = end[0] - start[0], end[1] - start[1]
    squared_length = dx * dx + dy * dy
    if squared_length == 0.0:
        return math.hypot(point[0] - start[0], point[1] - start[1])
    projection = (
        (point[0] - start[0]) * dx + (point[1] - start[1]) * dy
    ) / squared_length
    projection = min(1.0, max(0.0, projection))
    return math.hypot(
        point[0] - (start[0] + projection * dx),
        point[1] - (start[1] + projection * dy),
    )


def _de_casteljau(points: list[tuple[float, float]], t: float) -> tuple[float, float]:
    level = points
    while len(level) > 1:
        level = [
            ((1.0 - t) * a[0] + t * b[0], (1.0 - t) * a[1] + t * b[1])
            for a, b in pairwise(level)
        ]
    return level[0]


def _expected(value: Any, answer: float, where: str) -> None:
    actual = _number(value, where)
    if not math.isclose(actual, answer, rel_tol=0.0, abs_tol=ORACLE_TOLERANCE):
        raise ValueError(f"{where} differs from independently computed result")


def _validate_case(case: Any) -> str:
    if not isinstance(case, dict):
        raise TypeError("case fields must form an object")
    case_id = case.get("id")
    if not isinstance(case_id, str) or CASE_ID.fullmatch(case_id) is None:
        raise ValueError("case id must be canonical lowercase hyphenated text")
    degree = case.get("degree")
    if type(degree) is not int or degree not in (2, 3):
        raise ValueError(f"{case_id}.degree must be 2 or 3")
    controls = case.get("control_points")
    if not isinstance(controls, list) or len(controls) != degree + 1:
        raise ValueError(f"{case_id}.control_points has wrong shape")
    points = [_point(value, f"{case_id}.control_points") for value in controls]
    tolerance = _number(case.get("tolerance"), f"{case_id}.tolerance")
    disposition = case.get("expected_disposition")
    if disposition == "invalid-tolerance":
        _fields(
            case,
            {"id", "degree", "control_points", "tolerance", "expected_disposition"},
            case_id,
        )
        if tolerance > 0.0:
            raise ValueError(f"{case_id} does not have invalid tolerance")
    elif disposition in ("chord", "subdivide"):
        termination = case.get("expected_termination")
        fields = {
            "id",
            "degree",
            "control_points",
            "tolerance",
            "expected_disposition",
            "witness_t",
            "expected_witness",
            "expected_witness_distance",
        }
        if termination is not None:
            fields.add("expected_termination")
        _fields(
            case,
            fields,
            case_id,
        )
        if tolerance <= 0.0:
            raise ValueError(f"{case_id} must have positive tolerance")
        t = _number(case["witness_t"], f"{case_id}.witness_t")
        if not 0.0 <= t <= 1.0:
            raise ValueError(f"{case_id}.witness_t must be in [0,1]")
        witness = _de_casteljau(points, t)
        expected_witness = _point(
            case["expected_witness"], f"{case_id}.expected_witness"
        )
        for actual, answer in zip(expected_witness, witness, strict=True):
            _expected(actual, answer, f"{case_id}.expected_witness")
        distance = _distance_to_segment(witness, points[0], points[-1])
        _expected(
            case["expected_witness_distance"],
            distance,
            f"{case_id}.expected_witness_distance",
        )
        control_bound = max(
            _distance_to_segment(point, points[0], points[-1]) for point in points[1:-1]
        )
        answer = "chord" if control_bound <= tolerance else "subdivide"
        if disposition != answer:
            raise ValueError(f"{case_id}.expected_disposition must be {answer}")
        if disposition == "subdivide" and distance <= tolerance:
            raise ValueError(f"{case_id}.witness distance must exceed tolerance")
        # For this arch, y(x)=2x(1-x). A chord covering parameter width w
        # has vertical midpoint residual w²/2. Every chord has slope at most
        # 2 in magnitude, so its Euclidean point-to-segment error is at least
        # w²/(2*sqrt(5)). At most 65,535 splits yield at most 65,536 chords,
        # one with w >= 1/65,536.
        if termination is not None and (
            termination != "budget-error"
            or degree != 2
            or points != [(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)]
            or disposition != "subdivide"
            or tolerance >= 1.0 / (2.0 * math.sqrt(5.0) * 65_536**2)
        ):
            raise ValueError(f"{case_id}.budget error lacks a lower-bound proof")
    else:
        raise ValueError(f"{case_id} has unsupported expected disposition")
    return case_id


def validate_document(document: Any) -> int:
    root = _fields(
        document,
        {"version", "absolute_tolerance", "max_depth", "max_subdivisions", "cases"},
        "corpus",
    )
    for field, required in (
        ("version", 1),
        ("max_depth", 32),
        ("max_subdivisions", 65_535),
    ):
        if type(root[field]) is not int or root[field] != required:
            raise ValueError(f"{field} must remain pinned to {required}")
    if (
        type(root["absolute_tolerance"]) not in (int, float)
        or root["absolute_tolerance"] != ORACLE_TOLERANCE
    ):
        raise ValueError("absolute_tolerance must remain pinned")
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
    print(f"validated {count} neutral G2D02 flattening cases")


if __name__ == "__main__":
    main()
