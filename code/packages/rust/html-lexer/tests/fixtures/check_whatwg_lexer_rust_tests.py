#!/usr/bin/env python3

"""Check Rust test coverage for focused WHATWG lexer fixtures."""

from __future__ import annotations

import argparse
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
TEST_DIR = FIXTURE_DIR.parent
FIXTURE_GLOB = "whatwg-*.json"


LEXER_EXECUTION_SNIPPETS = (
    "common::assert_lexer_case(",
    "common::assert_default_lexer_case(",
    "create_html_lexer()",
    "create_html_lexer_with_context(",
)

CASE_ITERATION_SNIPPETS = (
    "for case in &suite.cases",
    "for case in &suite.position_cases",
    "for entity in &suite.entities",
    "for entity in suite.entities.iter()",
)


def main() -> int:
    parse_args()
    fixture_paths = sorted(FIXTURE_DIR.glob(FIXTURE_GLOB))
    test_paths = sorted(TEST_DIR.glob("whatwg_*_test.rs"))

    errors: list[str] = []
    errors.extend(check_test_pairs(fixture_paths, test_paths))
    for fixture_path in fixture_paths:
        errors.extend(check_test_file(fixture_path))

    print("WHATWG lexer Rust tests")
    print(f"fixture files: {len(fixture_paths)}")
    print(f"rust tests:    {len(test_paths)}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check that every focused WHATWG lexer fixture has a matching Rust "
            "test that includes and parses the fixture, iterates over fixture "
            "cases, and exercises the HTML lexer harness."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for the generated fixture manifest.",
    )
    return parser.parse_args()


def check_test_pairs(fixture_paths: list[Path], test_paths: list[Path]) -> list[str]:
    fixture_names = {fixture_name(path) for path in fixture_paths}
    test_names = {test_name(path) for path in test_paths}

    errors: list[str] = []
    missing_tests = sorted(fixture_names - test_names)
    stale_tests = sorted(test_names - fixture_names)
    if missing_tests:
        errors.append(
            "WHATWG lexer fixtures without matching Rust tests:\n"
            + "\n".join(f"  {name}" for name in missing_tests)
        )
    if stale_tests:
        errors.append(
            "Rust WHATWG lexer tests without matching fixtures:\n"
            + "\n".join(f"  {name}" for name in stale_tests)
        )
    return errors


def check_test_file(fixture_path: Path) -> list[str]:
    name = fixture_name(fixture_path)
    test_path = TEST_DIR / f"whatwg_{name.replace('-', '_')}_test.rs"
    if not test_path.exists():
        return []

    text = test_path.read_text()
    errors: list[str] = []

    required_snippets = {
        "fixture include": f'include_str!("fixtures/{fixture_path.name}")',
        "fixture parser": "serde_json::from_str(",
        "load_suite helper": "fn load_suite()",
        "Rust test": "#[test]",
        "fixture parse expectation": "fixture should parse",
    }
    for label, snippet in required_snippets.items():
        if snippet not in text:
            errors.append(f"{test_path.name} is missing {label}: {snippet}")

    if not any(snippet in text for snippet in LEXER_EXECUTION_SNIPPETS):
        errors.append(
            f"{test_path.name} does not exercise the HTML lexer harness; "
            "expected one of: "
            + ", ".join(LEXER_EXECUTION_SNIPPETS)
        )

    if not any(snippet in text for snippet in CASE_ITERATION_SNIPPETS):
        errors.append(
            f"{test_path.name} does not iterate over fixture cases; "
            "expected one of: "
            + ", ".join(CASE_ITERATION_SNIPPETS)
        )

    return errors


def fixture_name(path: Path) -> str:
    return path.name.removeprefix("whatwg-").removesuffix(".json")


def test_name(path: Path) -> str:
    return path.name.removeprefix("whatwg_").removesuffix("_test.rs").replace("_", "-")


if __name__ == "__main__":
    raise SystemExit(main())
