#!/usr/bin/env python3

"""Check that the normalized html5lib tokenizer fixture covers the raw corpus."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from normalize_html5lib_fixtures import SUPPORTED_INITIAL_STATES, is_supported


FIXTURE_DIR = Path(__file__).resolve().parent
DEFAULT_RAW = FIXTURE_DIR / "upstream-html5lib-smoke.test"
DEFAULT_NORMALIZED = FIXTURE_DIR / "html5lib-smoke.json"
CASE_ID_RE = re.compile(r"^html5lib-smoke-(?P<index>[1-9]\d*)(?:-(?P<variant>[1-9]\d*))?$")


def main() -> int:
    args = parse_args()
    raw_path = Path(args.raw).expanduser().resolve()
    normalized_path = Path(args.normalized).expanduser().resolve()

    raw = load_json(raw_path)
    normalized = load_json(normalized_path)
    tests = require_list(raw, "tests", raw_path)
    cases = require_list(normalized, "cases", normalized_path)
    skipped = require_list(normalized, "skipped", normalized_path)

    expected = expected_ids(tests)
    actual_cases = collect_ids(cases, "case")
    actual_skipped = collect_ids(skipped, "skipped")

    print("html5lib tokenizer coverage")
    print(f"raw fixture:        {raw_path}")
    print(f"normalized fixture: {normalized_path}")
    print(f"raw cases:          {len(tests)}")
    print(f"normalized cases:   {len(cases)}")
    print(f"skipped cases:      {len(skipped)}")
    print(f"covered raw cases:  {len(covered_raw_indices(actual_cases.ids, actual_skipped.ids))}")

    errors: list[str] = []
    errors.extend(metadata_errors(normalized, raw_path))
    errors.extend(actual_cases.errors)
    errors.extend(actual_skipped.errors)

    duplicate_across_sections = sorted(actual_cases.ids & actual_skipped.ids)
    if duplicate_across_sections:
        errors.append(
            "ids present in both normalized cases and skipped cases:\n"
            + "\n".join(f"  {case_id}" for case_id in duplicate_across_sections)
        )

    missing_cases = sorted(expected.case_ids - actual_cases.ids)
    unexpected_cases = sorted(actual_cases.ids - expected.case_ids)
    missing_skipped = sorted(expected.skipped_ids - actual_skipped.ids)
    unexpected_skipped = sorted(actual_skipped.ids - expected.skipped_ids)

    if missing_cases:
        errors.append("missing normalized case ids:\n" + format_ids(missing_cases))
    if unexpected_cases:
        errors.append("unexpected normalized case ids:\n" + format_ids(unexpected_cases))
    if missing_skipped:
        errors.append("missing skipped case ids:\n" + format_ids(missing_skipped))
    if unexpected_skipped:
        errors.append("unexpected skipped case ids:\n" + format_ids(unexpected_skipped))

    reason_mismatches = skipped_reason_mismatches(skipped, expected.skipped_reasons)
    if reason_mismatches:
        errors.append(
            "skipped case reasons do not match the normalizer:\n"
            + "\n".join(
                f"  {case_id}: expected {expected_reason!r}, found {actual_reason!r}"
                for case_id, expected_reason, actual_reason in reason_mismatches
            )
        )

    missing_raw_indices = set(range(1, len(tests) + 1)) - covered_raw_indices(
        actual_cases.ids, actual_skipped.ids
    )
    if missing_raw_indices:
        errors.append(
            "raw tests without a normalized or skipped entry:\n"
            + "\n".join(f"  html5lib-smoke-{index}" for index in sorted(missing_raw_indices))
        )

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check that every raw html5lib tokenizer smoke test maps to the "
            "expected normalized fixture case or skipped-case marker."
        )
    )
    parser.add_argument(
        "--raw",
        default=str(DEFAULT_RAW),
        help="Raw html5lib-style tokenizer smoke fixture.",
    )
    parser.add_argument(
        "--normalized",
        default=str(DEFAULT_NORMALIZED),
        help="Normalized Venture tokenizer fixture to audit.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for the generated fixture manifest.",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise SystemExit(f"{path} should contain a JSON object")
    return value


def require_list(parent: dict[str, Any], key: str, path: Path) -> list[dict[str, Any]]:
    value = parent.get(key)
    if not isinstance(value, list):
        raise SystemExit(f"{path} should contain a `{key}` list")
    for index, item in enumerate(value, start=1):
        if not isinstance(item, dict):
            raise SystemExit(f"{path} `{key}` entry {index} should be an object")
    return value


class ExpectedIds:
    def __init__(self) -> None:
        self.case_ids: set[str] = set()
        self.skipped_ids: set[str] = set()
        self.skipped_reasons: dict[str, str] = {}


def expected_ids(tests: list[dict[str, Any]]) -> ExpectedIds:
    expected = ExpectedIds()
    for index, test in enumerate(tests, start=1):
        base_id = f"html5lib-smoke-{index}"
        supported, reason = is_supported(test)
        if not supported:
            expected.skipped_ids.add(base_id)
            expected.skipped_reasons[base_id] = reason
            continue

        initial_states = test.get("initialStates", [])
        if len(initial_states) <= 1:
            expected.case_ids.add(base_id)
        else:
            expected.case_ids.update(
                f"{base_id}-{variant}" for variant in range(1, len(initial_states) + 1)
            )
    return expected


class CollectedIds:
    def __init__(self, ids: set[str], errors: list[str]) -> None:
        self.ids = ids
        self.errors = errors


def collect_ids(entries: list[dict[str, Any]], section: str) -> CollectedIds:
    ids: set[str] = set()
    duplicate_ids: set[str] = set()
    bad_ids: list[str] = []

    for entry in entries:
        case_id = entry.get("id")
        if not isinstance(case_id, str) or CASE_ID_RE.match(case_id) is None:
            bad_ids.append(str(case_id))
            continue
        if case_id in ids:
            duplicate_ids.add(case_id)
        ids.add(case_id)

    errors: list[str] = []
    if duplicate_ids:
        errors.append(
            f"duplicate {section} ids:\n" + format_ids(sorted(duplicate_ids))
        )
    if bad_ids:
        errors.append(f"malformed {section} ids:\n" + format_ids(sorted(bad_ids)))
    return CollectedIds(ids, errors)


def metadata_errors(normalized: dict[str, Any], raw_path: Path) -> list[str]:
    errors: list[str] = []
    if normalized.get("source") != raw_path.name:
        errors.append(
            f"normalized source should be {raw_path.name!r}, found {normalized.get('source')!r}"
        )
    if normalized.get("generator") != "normalize_html5lib_fixtures.py":
        errors.append(
            "normalized generator should be 'normalize_html5lib_fixtures.py', "
            f"found {normalized.get('generator')!r}"
        )

    supported_states = normalized.get("supported_initial_states")
    if supported_states != sorted(SUPPORTED_INITIAL_STATES):
        errors.append("normalized supported_initial_states drifted from the normalizer")
    return errors


def skipped_reason_mismatches(
    skipped: list[dict[str, Any]], expected_reasons: dict[str, str]
) -> list[tuple[str, str, str]]:
    mismatches: list[tuple[str, str, str]] = []
    for entry in skipped:
        case_id = entry.get("id")
        if not isinstance(case_id, str) or case_id not in expected_reasons:
            continue
        actual_reason = entry.get("reason")
        expected_reason = expected_reasons[case_id]
        if actual_reason != expected_reason:
            mismatches.append((case_id, expected_reason, str(actual_reason)))
    return mismatches


def covered_raw_indices(case_ids: set[str], skipped_ids: set[str]) -> set[int]:
    covered: set[int] = set()
    for case_id in case_ids | skipped_ids:
        match = CASE_ID_RE.match(case_id)
        if match is None:
            continue
        covered.add(int(match.group("index")))
    return covered


def format_ids(ids: list[str]) -> str:
    return "\n".join(f"  {case_id}" for case_id in ids)


if __name__ == "__main__":
    raise SystemExit(main())
