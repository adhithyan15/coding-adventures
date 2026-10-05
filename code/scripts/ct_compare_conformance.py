"""Validate CT01's neutral oracle without calling a language implementation.

The fixture is a published contract, not a snapshot of any package's output.
This validator recomputes its functional answers and its *logical* public-work
counts directly from canonical input text. It makes no timing assertion.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_CORPUS = ROOT / "code" / "specs" / "fixtures" / "ct-compare-v1" / "cases.json"
BYTE_HEX = re.compile(r"(?:[0-9a-f]{2})*\Z")
U64_HEX = re.compile(r"[0-9a-f]{16}\Z")
CASE_ID = re.compile(r"[a-z][a-z0-9-]*\Z")
ERROR_ID = "CT_SELECT_LENGTH_MISMATCH"


def _object_without_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """JSON permits some parsers to silently keep the last duplicate key.

    A conformance corpus must not admit two possible interpretations, so reject
    duplicates while decoding, before any semantic validation occurs.
    """

    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_json(text: str) -> Any:
    return json.loads(text, object_pairs_hook=_object_without_duplicate_keys)


def _fields(value: Any, required: set[str], where: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != required:
        raise ValueError(f"{where} fields must be exactly {sorted(required)}")
    return value


def _byte_hex(value: Any, where: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) > 8192
        or BYTE_HEX.fullmatch(value) is None
    ):
        raise ValueError(f"{where} must be canonical lowercase even-length hex")
    return value


def _u64_hex(value: Any, where: str) -> str:
    if not isinstance(value, str) or U64_HEX.fullmatch(value) is None:
        raise ValueError(f"{where} must be exactly 16 lowercase u64 hex digits")
    return value


def _public_work(
    positions: int = 0, *, select: bool = False, u64: bool = False
) -> dict[str, int]:
    """Count logical units only; these are not machine instructions or cycles."""

    return {
        "byte_positions": positions,
        "byte_reads": 2 * positions,
        "byte_writes": positions if select else 0,
        "u64_bits": 64 if u64 else 0,
    }


def _validate_case(case: Any) -> None:
    if not isinstance(case, dict):
        raise TypeError("case fields must be an object")
    operation = case.get("operation")
    common = {"id", "operation", "expected", "expected_work"}
    if operation == "eq":
        required = common | {"left_hex", "right_hex"}
    elif operation == "eq_fixed":
        required = common | {"fixed_size_bytes", "left_hex", "right_hex"}
    elif operation == "select_bytes":
        required = common | {"left_hex", "right_hex", "choice"}
    elif operation == "eq_u64":
        required = common | {"left_u64", "right_u64"}
    else:
        raise ValueError(f"unknown operation: {operation}")
    _fields(case, required, "case")
    case_id = case["id"]
    if (
        not isinstance(case_id, str)
        or len(case_id) > 80
        or CASE_ID.fullmatch(case_id) is None
    ):
        raise ValueError("case id must be canonical kebab-case")

    if operation == "eq_u64":
        left = _u64_hex(case["left_u64"], "left_u64")
        right = _u64_hex(case["right_u64"], "right_u64")
        expected = {"equal": int(left, 16) == int(right, 16)}
        work = _public_work(u64=True)
    else:
        left = _byte_hex(case["left_hex"], "left_hex")
        right = _byte_hex(case["right_hex"], "right_hex")
        left_bytes = bytes.fromhex(left)
        right_bytes = bytes.fromhex(right)
        same_length = len(left_bytes) == len(right_bytes)
        positions = len(left_bytes) if same_length else 0
        if operation == "eq_fixed":
            width = case["fixed_size_bytes"]
            if (
                type(width) is not int
                or not 0 <= width <= 4096
                or (len(left_bytes) != width or len(right_bytes) != width)
            ):
                raise ValueError("fixed_size_bytes must equal both input lengths")
        if operation == "select_bytes":
            if type(case["choice"]) is not bool:
                raise ValueError("choice must be boolean")
            expected = (
                {"output_hex": left if case["choice"] else right, "fresh_output": True}
                if same_length
                else {"error_id": ERROR_ID}
            )
            work = _public_work(positions, select=same_length)
        else:
            expected = {"equal": left_bytes == right_bytes}
            work = _public_work(positions)

    if (
        type(case["expected"]) is not dict
        or case["expected"] != expected
        or (
            any(
                type(value) is not type(expected[key])
                for key, value in case["expected"].items()
            )
        )
    ):
        raise ValueError(f"{case_id} expected result disagrees with independent oracle")
    observed_work = _fields(case["expected_work"], set(work), "expected_work")
    if (
        any(type(observed_work[key]) is not int for key in work)
        or observed_work != work
    ):
        raise ValueError(f"{case_id} expected_work disagrees with public input shape")


def validate_document(document: Any) -> int:
    """Return the validated case count or raise ValueError on any ambiguity."""

    root = _fields(document, {"schema_version", "profile", "cases"}, "corpus")
    if type(root["schema_version"]) is not int or root["schema_version"] != 1:
        raise ValueError("schema_version must be 1")
    if root["profile"] != "ct-compare-v1":
        raise ValueError("profile must be ct-compare-v1")
    cases = root["cases"]
    if not isinstance(cases, list) or not 1 <= len(cases) <= 256:
        raise ValueError("cases must contain 1..256 entries")
    seen: set[str] = set()
    for case in cases:
        _validate_case(case)
        if case["id"] in seen:
            raise ValueError(f"duplicate case id: {case['id']}")
        seen.add(case["id"])
    return len(cases)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=pathlib.Path, default=DEFAULT_CORPUS)
    args = parser.parse_args()
    count = validate_document(parse_json(args.corpus.read_text(encoding="utf-8")))
    print(f"ct-compare-v1: {count} cases valid")


if __name__ == "__main__":
    main()
