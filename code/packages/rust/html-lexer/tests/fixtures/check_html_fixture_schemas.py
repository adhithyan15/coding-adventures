#!/usr/bin/env python3

"""Check checked-in HTML lexer/parser fixture JSON schemas."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


FIXTURE_DIR = Path(__file__).resolve().parent
RUST_DIR = FIXTURE_DIR.parents[2]
PARSER_FIXTURE_DIR = RUST_DIR / "html-parser" / "tests" / "fixtures"
PARSER_SOURCE_FIXTURE = "html5lib-tree-construction-smoke.dat"
NUMERIC_REFERENCE_FIXTURE = "whatwg-numeric-references.json"
INPUT_STREAM_FIXTURE = "whatwg-input-stream.json"
CHUNK_BOUNDARY_FIXTURE = "whatwg-chunk-boundaries.json"


@dataclass(frozen=True)
class FixtureStats:
    fixture_count: int
    case_count: int


def main() -> int:
    parse_args()
    errors, stats = check_fixture_schemas()

    print("HTML fixture JSON schemas")
    print(f"fixture files: {stats.fixture_count}")
    print(f"cases: {stats.case_count}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check the JSON schemas used by checked-in HTML lexer/parser "
            "fixture corpora."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for generated-fixture stale-check manifests.",
    )
    return parser.parse_args()


def check_fixture_schemas() -> tuple[list[str], FixtureStats]:
    errors: list[str] = []
    fixture_count = 0
    case_count = 0

    for fixture_path in fixture_json_files():
        data = read_json_object(fixture_path, errors)
        if data is None or "cases" not in data:
            continue

        relative_path = relative_fixture(fixture_path)
        cases = data.get("cases")
        fixture_count += 1
        errors.extend(check_top_level_schema(relative_path, data))

        if not isinstance(cases, list):
            errors.append(f"{relative_path}: cases must be a list")
            continue

        case_count += len(cases)
        for index, case in enumerate(cases):
            if not isinstance(case, dict):
                errors.append(f"{relative_path}: cases[{index}] must be an object")
                continue
            if fixture_path.parent == PARSER_FIXTURE_DIR:
                errors.extend(check_parser_audit_case(relative_path, index, case))
            else:
                errors.extend(check_lexer_case(relative_path, fixture_path.name, index, case))

    return errors, FixtureStats(fixture_count=fixture_count, case_count=case_count)


def fixture_json_files() -> list[Path]:
    fixture_dirs = (FIXTURE_DIR, PARSER_FIXTURE_DIR)
    return sorted(
        fixture_path
        for fixture_dir in fixture_dirs
        for fixture_path in fixture_dir.glob("*.json")
        if fixture_path.is_file()
    )


def read_json_object(fixture_path: Path, errors: list[str]) -> dict[str, Any] | None:
    relative_path = relative_fixture(fixture_path)
    try:
        data = json.loads(fixture_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        errors.append(f"{relative_path}: invalid JSON: {exc}")
        return None

    if not isinstance(data, dict):
        errors.append(f"{relative_path}: fixture must be a JSON object")
        return None
    return data


def check_top_level_schema(relative_path: str, data: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    require_non_empty_string(relative_path, data, "format", errors)
    require_non_empty_string(relative_path, data, "description", errors)

    if relative_path.startswith("html-parser/"):
        if data.get("source_fixture") != PARSER_SOURCE_FIXTURE:
            errors.append(
                f"{relative_path}: source_fixture must be {PARSER_SOURCE_FIXTURE!r}"
            )
        if not isinstance(data.get("case_count"), int):
            errors.append(f"{relative_path}: case_count must be an integer")
        if not isinstance(data.get("counts_by_axis"), dict):
            errors.append(f"{relative_path}: counts_by_axis must be an object")
    elif data.get("format") == "venture-html-lexer-fixtures/v1":
        require_non_empty_string(relative_path, data, "suite", errors)

    return errors


def check_lexer_case(
    relative_path: str,
    fixture_name: str,
    index: int,
    case: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    case_path = f"{relative_path}: cases[{index}]"

    if fixture_name == NUMERIC_REFERENCE_FIXTURE:
        require_integer(case_path, case, "value", errors)
        require_non_empty_string(case_path, case, "characters", errors)
        require_string(case_path, case, "decimal", errors)
        require_string(case_path, case, "decimal_missing_semicolon", errors)
        require_string(case_path, case, "hex", errors)
        require_string(case_path, case, "hex_missing_semicolon", errors)
        require_int_list(case_path, case, "codepoints", errors)
        require_optional_string_list(case_path, case, "diagnostics", errors)
        return errors

    require_string(case_path, case, "input", errors)
    require_optional_non_empty_string(case_path, case, "description", errors)

    if fixture_name == INPUT_STREAM_FIXTURE:
        require_string(case_path, case, "normalized", errors)
    elif fixture_name == CHUNK_BOUNDARY_FIXTURE:
        require_int_list(case_path, case, "split_points", errors)
        check_split_points(case_path, case, errors)
    else:
        require_string_list(case_path, case, "tokens", errors)
        require_optional_string_list(case_path, case, "diagnostics", errors)

    for field in (
        "initial_state",
        "last_start_tag",
        "current_end_tag",
        "temporary_buffer",
        "return_state",
    ):
        require_optional_non_empty_string(case_path, case, field, errors)
    require_optional_string(case_path, case, "current_comment", errors)
    for field in ("start_tag", "current_doctype"):
        if field in case and not isinstance(case[field], dict):
            errors.append(f"{case_path}.{field} must be an object")

    return errors


def check_parser_audit_case(
    relative_path: str,
    index: int,
    case: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    case_path = f"{relative_path}: cases[{index}]"

    for field in ("id", "axis", "reason", "source"):
        require_non_empty_string(case_path, case, field, errors)
    for field in ("context", "scripting"):
        require_optional_non_empty_string(case_path, case, field, errors)

    return errors


def check_split_points(
    case_path: str,
    case: dict[str, Any],
    errors: list[str],
) -> None:
    input_value = case.get("input")
    split_points = case.get("split_points")
    if not isinstance(input_value, str) or not isinstance(split_points, list):
        return

    input_length = len(input_value)
    invalid_points = [
        point
        for point in split_points
        if not isinstance(point, int) or point < 0 or point > input_length
    ]
    if invalid_points:
        errors.append(
            f"{case_path}.split_points contains positions outside input length "
            f"{input_length}: {invalid_points!r}"
        )


def require_non_empty_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    value = data.get(field)
    if not isinstance(value, str) or not value:
        errors.append(f"{path}.{field} must be a non-empty string")


def require_optional_non_empty_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_non_empty_string(path, data, field, errors)


def require_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if not isinstance(data.get(field), str):
        errors.append(f"{path}.{field} must be a string")


def require_optional_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_string(path, data, field, errors)


def require_integer(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if not isinstance(data.get(field), int):
        errors.append(f"{path}.{field} must be an integer")


def require_string_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    value = data.get(field)
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        errors.append(f"{path}.{field} must be a list of strings")


def require_optional_string_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_string_list(path, data, field, errors)


def require_int_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    value = data.get(field)
    if not isinstance(value, list) or not all(isinstance(item, int) for item in value):
        errors.append(f"{path}.{field} must be a list of integers")


def relative_fixture(path: Path) -> str:
    try:
        return str(path.relative_to(RUST_DIR))
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
