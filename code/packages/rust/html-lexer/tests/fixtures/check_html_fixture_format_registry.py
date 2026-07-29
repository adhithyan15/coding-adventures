#!/usr/bin/env python3

"""Check checked-in HTML fixture JSON format strings against a registry."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


FIXTURE_DIR = Path(__file__).resolve().parent
RUST_DIR = FIXTURE_DIR.parents[2]
PARSER_FIXTURE_DIR = RUST_DIR / "html-parser" / "tests" / "fixtures"


@dataclass(frozen=True)
class FixtureFormat:
    relative_path: str
    format: str
    category: str


@dataclass(frozen=True)
class RegistryStats:
    registered_count: int
    format_fixture_count: int


FORMAT_REGISTRY: tuple[FixtureFormat, ...] = (
    FixtureFormat(
        "html-lexer/tests/fixtures/html-skeleton.json",
        "venture-html-lexer-fixtures/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/html1.json",
        "venture-html-lexer-fixtures/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/html5lib-smoke.json",
        "venture-html-lexer-fixtures/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-attribute-boundaries.json",
        "whatwg-html-tokenizer-attribute-boundaries/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-attribute-edges.json",
        "whatwg-html-tokenizer-attribute-edges/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-cdata-boundaries.json",
        "whatwg-html-tokenizer-cdata-boundaries/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-character-reference-boundaries.json",
        "whatwg-html-tokenizer-character-reference-boundaries/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-chunk-boundaries.json",
        "whatwg-html-tokenizer-chunk-boundaries/v1",
        "lexer-chunk-boundary",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-comment-boundaries.json",
        "whatwg-html-tokenizer-comment-boundaries/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-doctype-boundaries.json",
        "whatwg-html-tokenizer-doctype-boundaries/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-entities.json",
        "whatwg-html-entities/v1",
        "lexer-entities",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-eof-recovery.json",
        "whatwg-html-tokenizer-eof-recovery/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-input-stream.json",
        "whatwg-html-input-stream-preprocessing/v1",
        "lexer-input-stream",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-markup-declarations.json",
        "whatwg-html-tokenizer-markup-declarations/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-numeric-references.json",
        "whatwg-html-numeric-character-references/v1",
        "lexer-numeric-reference",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-script-escape-boundaries.json",
        "whatwg-html-tokenizer-script-escape-boundaries/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-tag-open-recovery.json",
        "whatwg-html-tokenizer-tag-open-recovery/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-text-mode-boundaries.json",
        "whatwg-html-tokenizer-text-mode-boundaries/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-lexer/tests/fixtures/whatwg-text-mode-delimiters.json",
        "whatwg-html-tokenizer-text-mode-delimiters/v1",
        "lexer-token",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/html-browser-readiness.json",
        "venture-html-browser-readiness/v1",
        "parser-browser-readiness",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/html-browser-content-tree.json",
        "venture-html-browser-content-tree/v1",
        "parser-browser-content-tree",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/html-browser-render-tree.json",
        "venture-html-browser-render-tree/v1",
        "parser-browser-render-tree",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-block-boundary-audit.json",
        "whatwg-html-block-boundary-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-character-reference-audit.json",
        "whatwg-html-character-reference-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-document-shell-audit.json",
        "whatwg-html-document-shell-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-foreign-audit.json",
        "whatwg-html-foreign-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-form-interactive-audit.json",
        "whatwg-html-form-interactive-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-formatting-audit.json",
        "whatwg-html-formatting-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-fragment-context-audit.json",
        "whatwg-html-fragment-context-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-frameset-audit.json",
        "whatwg-html-frameset-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-head-body-audit.json",
        "whatwg-html-head-body-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-legacy-element-audit.json",
        "whatwg-html-legacy-element-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-list-item-audit.json",
        "whatwg-html-list-item-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-misc-recovery-audit.json",
        "whatwg-html-misc-recovery-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-noscript-audit.json",
        "whatwg-html-noscript-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-paragraph-audit.json",
        "whatwg-html-paragraph-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-processing-instruction-audit.json",
        "whatwg-html-processing-instruction-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-ruby-audit.json",
        "whatwg-html-ruby-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-select-list-audit.json",
        "whatwg-html-select-list-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-table-audit.json",
        "whatwg-html-table-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-template-audit.json",
        "whatwg-html-template-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-text-control-audit.json",
        "whatwg-html-text-control-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-tree-insertion-audit.json",
        "whatwg-html-tree-insertion-audit/v1",
        "parser-audit",
    ),
    FixtureFormat(
        "html-parser/tests/fixtures/whatwg-void-element-audit.json",
        "whatwg-html-void-element-audit/v1",
        "parser-audit",
    ),
)


def main() -> int:
    parse_args()
    errors, stats = check_fixture_format_registry()

    print("HTML fixture format registry")
    print(f"registered fixture files: {stats.registered_count}")
    print(f"format-bearing fixture files: {stats.format_fixture_count}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check that every checked-in HTML lexer/parser JSON fixture format "
            "string is explicitly registered."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for generated-fixture stale-check manifests.",
    )
    return parser.parse_args()


def check_fixture_format_registry() -> tuple[list[str], RegistryStats]:
    errors: list[str] = []
    registry = registry_by_path(errors)
    format_fixtures = format_bearing_fixture_json(errors)

    registry_paths = set(registry)
    fixture_paths = set(format_fixtures)

    for relative_path in sorted(registry_paths - fixture_paths):
        errors.append(f"{relative_path}: registered fixture file is missing or has no format")
    for relative_path in sorted(fixture_paths - registry_paths):
        errors.append(f"{relative_path}: format-bearing fixture file is not registered")

    for relative_path in sorted(registry_paths & fixture_paths):
        expected = registry[relative_path]
        data = format_fixtures[relative_path]
        actual_format = data.get("format")
        if actual_format != expected.format:
            errors.append(
                f"{relative_path}: format={actual_format!r} "
                f"does not match registry value {expected.format!r}"
            )
        errors.extend(check_category_contract(relative_path, expected.category, data))

    stats = RegistryStats(
        registered_count=len(registry),
        format_fixture_count=len(format_fixtures),
    )
    return errors, stats


def registry_by_path(errors: list[str]) -> dict[str, FixtureFormat]:
    registry: dict[str, FixtureFormat] = {}
    for entry in FORMAT_REGISTRY:
        if entry.relative_path in registry:
            errors.append(f"{entry.relative_path}: duplicate registry entry")
        registry[entry.relative_path] = entry
    return registry


def format_bearing_fixture_json(
    errors: list[str],
) -> dict[str, dict[str, Any]]:
    fixtures: dict[str, dict[str, Any]] = {}

    for fixture_path in fixture_json_files():
        data = read_json_object(fixture_path, errors)
        if data is None or "format" not in data:
            continue
        relative_path = relative_fixture(fixture_path)
        fixtures[relative_path] = data

    return fixtures


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


def check_category_contract(
    relative_path: str,
    category: str,
    data: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    if category == "lexer-token":
        require_cases(relative_path, data, errors)
        require_case_field(relative_path, data, "tokens", list, errors)
    elif category == "lexer-input-stream":
        require_cases(relative_path, data, errors)
        require_case_field(relative_path, data, "normalized", str, errors)
    elif category == "lexer-chunk-boundary":
        require_cases(relative_path, data, errors)
        require_case_field(relative_path, data, "split_points", list, errors)
    elif category == "lexer-numeric-reference":
        require_cases(relative_path, data, errors)
        require_case_field(relative_path, data, "value", int, errors)
    elif category == "lexer-entities":
        require_top_level(relative_path, data, "entities", list, errors)
    elif category == "parser-audit":
        require_cases(relative_path, data, errors)
        require_top_level(relative_path, data, "source_fixture", str, errors)
        require_top_level(relative_path, data, "case_count", int, errors)
    elif category == "parser-browser-readiness":
        require_cases(relative_path, data, errors)
        require_top_level(relative_path, data, "suite", str, errors)
        require_case_field(relative_path, data, "expected", dict, errors)
    elif category == "parser-browser-content-tree":
        require_cases(relative_path, data, errors)
        require_top_level(relative_path, data, "suite", str, errors)
        require_case_field(relative_path, data, "expected", dict, errors)
    elif category == "parser-browser-render-tree":
        require_cases(relative_path, data, errors)
        require_top_level(relative_path, data, "suite", str, errors)
        require_case_field(relative_path, data, "expected", dict, errors)
    else:
        errors.append(f"{relative_path}: unknown registry category {category!r}")
    return errors


def require_cases(
    relative_path: str,
    data: dict[str, Any],
    errors: list[str],
) -> None:
    require_top_level(relative_path, data, "cases", list, errors)


def require_top_level(
    relative_path: str,
    data: dict[str, Any],
    field: str,
    expected_type: type,
    errors: list[str],
) -> None:
    value = data.get(field)
    if not isinstance(value, expected_type):
        errors.append(
            f"{relative_path}: {field} must be {expected_type.__name__} "
            f"for registered format category"
        )


def require_case_field(
    relative_path: str,
    data: dict[str, Any],
    field: str,
    expected_type: type,
    errors: list[str],
) -> None:
    cases = data.get("cases")
    if not isinstance(cases, list):
        return

    for index, case in enumerate(cases):
        if not isinstance(case, dict):
            continue
        value = case.get(field)
        if not isinstance(value, expected_type):
            errors.append(
                f"{relative_path}: cases[{index}].{field} must be "
                f"{expected_type.__name__} for registered format category"
            )


def relative_fixture(path: Path) -> str:
    try:
        return str(path.relative_to(RUST_DIR))
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
