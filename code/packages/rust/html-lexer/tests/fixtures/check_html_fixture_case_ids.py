#!/usr/bin/env python3

"""Check stable case identities in checked-in HTML lexer/parser fixtures."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any


FIXTURE_DIR = Path(__file__).resolve().parent
RUST_DIR = FIXTURE_DIR.parents[2]
PARSER_FIXTURE_DIR = RUST_DIR / "html-parser" / "tests" / "fixtures"
NUMERIC_REFERENCE_FIXTURE = "whatwg-numeric-references.json"


@dataclass(frozen=True)
class FixtureStats:
    fixture_count: int
    case_count: int


def main() -> int:
    parse_args()
    errors, stats = check_fixture_case_ids()

    print("HTML fixture case identities")
    print(f"fixture files: {stats.fixture_count}")
    print(f"cases: {stats.case_count}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check stable, unique case identity fields in checked-in HTML "
            "lexer/parser JSON fixtures."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for generated-fixture stale-check manifests.",
    )
    return parser.parse_args()


def check_fixture_case_ids() -> tuple[list[str], FixtureStats]:
    errors: list[str] = []
    fixture_count = 0
    case_count = 0

    for fixture_path in fixture_json_files():
        relative_path = relative_fixture(fixture_path)
        data = read_json_object(fixture_path, errors)
        if data is None or "cases" not in data:
            continue

        cases = data["cases"]
        fixture_count += 1
        if not isinstance(cases, list):
            errors.append(f"{relative_path}: cases must be a list")
            continue

        case_count += len(cases)
        identity_errors = check_case_identities(relative_path, fixture_path.name, cases)
        audit_errors = check_parser_audit_counts(relative_path, data, cases)
        errors.extend(identity_errors)
        errors.extend(audit_errors)

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


def check_case_identities(
    relative_path: str,
    fixture_name: str,
    cases: list[Any],
) -> list[str]:
    errors: list[str] = []
    identities: list[str | int] = []

    for index, case in enumerate(cases):
        if not isinstance(case, dict):
            errors.append(f"{relative_path}: cases[{index}] must be an object")
            continue

        identity = case_identity(relative_path, fixture_name, index, case, errors)
        if identity is not None:
            identities.append(identity)

    duplicate_identities = [
        identity for identity, count in Counter(identities).items() if count > 1
    ]
    if duplicate_identities:
        formatted = ", ".join(str(identity) for identity in sorted(duplicate_identities))
        errors.append(f"{relative_path}: duplicate case identities: {formatted}")

    return errors


def case_identity(
    relative_path: str,
    fixture_name: str,
    index: int,
    case: dict[str, Any],
    errors: list[str],
) -> str | int | None:
    if fixture_name == NUMERIC_REFERENCE_FIXTURE:
        value = case.get("value")
        if isinstance(value, int):
            return value
        errors.append(f"{relative_path}: cases[{index}].value must be an integer")
        return None

    case_id = case.get("id")
    if isinstance(case_id, str) and case_id:
        return case_id
    errors.append(f"{relative_path}: cases[{index}].id must be a non-empty string")
    return None


def check_parser_audit_counts(
    relative_path: str,
    data: dict[str, Any],
    cases: list[Any],
) -> list[str]:
    errors: list[str] = []

    if "case_count" in data and data["case_count"] != len(cases):
        errors.append(
            f"{relative_path}: case_count={data['case_count']!r} "
            f"does not match {len(cases)} cases"
        )

    if "counts_by_axis" in data:
        expected_counts = data["counts_by_axis"]
        if not isinstance(expected_counts, dict):
            errors.append(f"{relative_path}: counts_by_axis must be an object")
            return errors

        axis_counts = Counter(
            case.get("axis")
            for case in cases
            if isinstance(case, dict) and isinstance(case.get("axis"), str)
        )
        actual_counts = dict(sorted(axis_counts.items()))
        if expected_counts != actual_counts:
            errors.append(
                f"{relative_path}: counts_by_axis={expected_counts!r} "
                f"does not match {actual_counts!r}"
            )

    return errors


def relative_fixture(path: Path) -> str:
    try:
        return str(path.relative_to(RUST_DIR))
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
