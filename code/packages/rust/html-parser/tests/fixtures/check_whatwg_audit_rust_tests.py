#!/usr/bin/env python3

"""Check Rust test coverage for focused WHATWG parser audit fixtures."""

from __future__ import annotations

import argparse
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
TEST_DIR = FIXTURE_DIR.parent
AUDIT_GLOB = "whatwg-*-audit.json"


def main() -> int:
    parse_args()
    audit_paths = sorted(FIXTURE_DIR.glob(AUDIT_GLOB))
    test_paths = sorted(TEST_DIR.glob("whatwg_*_audit_test.rs"))

    errors: list[str] = []
    errors.extend(check_test_pairs(audit_paths, test_paths))
    for audit_path in audit_paths:
        errors.extend(check_test_file(audit_path))

    print("WHATWG parser audit Rust tests")
    print(f"audit files: {len(audit_paths)}")
    print(f"rust tests:  {len(test_paths)}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check that every focused WHATWG parser audit fixture has a "
            "matching Rust test that parses the fixture and replays its cases "
            "through the parser DOM-dump harness, with a focused executable "
            "evidence guard for representative cases."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for the generated fixture manifest.",
    )
    return parser.parse_args()


def check_test_pairs(audit_paths: list[Path], test_paths: list[Path]) -> list[str]:
    audit_names = {audit_name(path) for path in audit_paths}
    test_names = {test_name(path) for path in test_paths}

    errors: list[str] = []
    missing_tests = sorted(audit_names - test_names)
    stale_tests = sorted(test_names - audit_names)
    if missing_tests:
        errors.append(
            "audit fixtures without matching Rust tests:\n"
            + "\n".join(f"  {name}" for name in missing_tests)
        )
    if stale_tests:
        errors.append(
            "Rust audit tests without matching fixtures:\n"
            + "\n".join(f"  {name}" for name in stale_tests)
        )
    return errors


def check_test_file(audit_path: Path) -> list[str]:
    name = audit_name(audit_path)
    test_path = TEST_DIR / f"whatwg_{name.replace('-', '_')}_audit_test.rs"
    if not test_path.exists():
        return []

    text = test_path.read_text()
    rust_name = name.replace("-", "_")
    errors: list[str] = []

    required_snippets = {
        "common DOM dump helpers": (
            "use common::{actual_dom_dump_for_tree_case, "
            "parse_tree_construction_cases};"
        ),
        "smoke fixture include": (
            'include_str!("fixtures/html5lib-tree-construction-smoke.dat")'
        ),
        "audit fixture include": f'include_str!("fixtures/{audit_path.name}")',
        "fixture parse test": f"fn whatwg_{rust_name}_audit_fixture_parses()",
        "case replay test": (
            f"fn whatwg_{rust_name}_audit_cases_match_parser_dom_dump()"
        ),
        "source fixture assertion": (
            'assert_eq!(suite.source_fixture, "html5lib-tree-construction-smoke.dat")'
        ),
        "case count assertion": "assert_eq!(suite.case_count, suite.cases.len())",
        "case id metadata assertion": "!case.id.is_empty()",
        "case axis metadata assertion": "!case.axis.is_empty()",
        "case reason metadata assertion": "!case.reason.is_empty()",
        "smoke source lookup": (
            ".unwrap_or_else(|| panic!(\"case `{}` should exist in smoke fixture\""
        ),
        "parser DOM dump": "actual_dom_dump_for_tree_case(source_case)",
        "expected DOM dump comparison": "actual, source_case.document",
    }

    for label, snippet in required_snippets.items():
        if snippet not in text:
            errors.append(f"{test_path.name} is missing {label}: {snippet}")

    evidence_guards = [
        f"fn whatwg_{rust_name}_audit_tracks_post_parse_repair_evidence()",
        f"fn whatwg_{rust_name}_audit_tracks_executable_evidence()",
    ]
    if not any(snippet in text for snippet in evidence_guards):
        errors.append(
            f"{test_path.name} is missing executable evidence guard: "
            + " or ".join(evidence_guards)
        )

    return errors


def audit_name(path: Path) -> str:
    return path.name.removeprefix("whatwg-").removesuffix("-audit.json")


def test_name(path: Path) -> str:
    return (
        path.name.removeprefix("whatwg_")
        .removesuffix("_audit_test.rs")
        .replace("_", "-")
    )


if __name__ == "__main__":
    raise SystemExit(main())
