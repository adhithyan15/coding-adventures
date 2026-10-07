"""Validate G2D00/G2D03 fixtures without consulting a package implementation.

The cases are a contract, not captured output. Recompute each answer from the
small published formulas, reject ambiguous transport, and never turn an
implementation's current behavior into its own expected value.
"""

from __future__ import annotations

import json
import math
import pathlib
import re
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


def _expected(value: Any, answer: tuple[float, ...], where: str) -> None:
    published = _tuple(value, len(answer), where)
    if any(
        not math.isclose(got, want, rel_tol=0.0, abs_tol=ORACLE_TOLERANCE)
        for got, want in zip(published, answer, strict=True)
    ):
        raise ValueError(f"{where} differs from independently computed expected result")


def _validate_case(case: Any) -> str:
    if not isinstance(case, dict):
        raise ValueError("case fields must form an object")
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
    print(f"validated {count} neutral G2D00/G2D03 cases")


if __name__ == "__main__":
    main()
