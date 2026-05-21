#!/usr/bin/env python3

"""Audit WHATWG lexer fixture metadata, generators, and manifest wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import check_generated_html_fixtures as manifest


FIXTURE_DIR = Path(__file__).resolve().parent
EXTERNAL_SOURCE_GENERATORS = {
    "generate_whatwg_entities_fixture.py",
}
SPECIAL_FORMATS = {
    "entities": "whatwg-html-entities/v1",
    "input-stream": "whatwg-html-input-stream-preprocessing/v1",
    "numeric-references": "whatwg-html-numeric-character-references/v1",
}
REQUIRED_CASE_FIELDS = {
    "description",
    "input",
}


def main() -> int:
    parse_args()
    errors: list[str] = []
    fixture_paths = sorted(FIXTURE_DIR.glob("whatwg-*.json"))

    if not fixture_paths:
        errors.append("no WHATWG lexer fixtures found")

    generator_errors = check_generator_pairs(fixture_paths)
    manifest_errors = check_manifest_wiring()
    fixture_errors, stats = check_fixture_metadata(fixture_paths)
    errors.extend(generator_errors)
    errors.extend(manifest_errors)
    errors.extend(fixture_errors)

    if errors:
        print("WHATWG lexer fixture metadata check failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(
        "WHATWG lexer fixture metadata ok: "
        f"{len(fixture_paths)} fixtures, "
        f"{stats['case_count']} cases, "
        f"{stats['entity_count']} entities"
    )
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check WHATWG lexer fixture metadata and manifest coverage."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for generated-fixture stale-check manifests.",
    )
    return parser.parse_args()


def check_generator_pairs(fixture_paths: list[Path]) -> list[str]:
    fixture_names = {fixture_name(path) for path in fixture_paths}
    generator_names = {
        generator_fixture_name(path)
        for path in FIXTURE_DIR.glob("generate_whatwg_*_fixture.py")
    }
    errors: list[str] = []

    missing_generators = fixture_names - generator_names
    if missing_generators:
        errors.append(
            "fixtures without matching generators: "
            + ", ".join(sorted(missing_generators))
        )

    missing_fixtures = generator_names - fixture_names
    if missing_fixtures:
        errors.append(
            "generators without matching fixtures: "
            + ", ".join(sorted(missing_fixtures))
        )

    return errors


def check_manifest_wiring() -> list[str]:
    checked_scripts = {
        Path(check.command[0]).name for check in manifest.default_checks()
    }
    generator_scripts = {
        path.name for path in FIXTURE_DIR.glob("generate_whatwg_*_fixture.py")
    }
    self_contained_generators = generator_scripts - EXTERNAL_SOURCE_GENERATORS
    errors: list[str] = []

    missing_generators = self_contained_generators - checked_scripts
    if missing_generators:
        errors.append(
            "default manifest omits lexer generator checks: "
            + ", ".join(sorted(missing_generators))
        )

    external_generators = checked_scripts & EXTERNAL_SOURCE_GENERATORS
    if external_generators:
        errors.append(
            "default manifest includes upstream-source generator checks: "
            + ", ".join(sorted(external_generators))
        )

    if Path(__file__).name not in checked_scripts:
        errors.append(f"default manifest omits {Path(__file__).name}")

    return errors


def check_fixture_metadata(fixture_paths: list[Path]) -> tuple[list[str], dict[str, int]]:
    errors: list[str] = []
    stats = {
        "case_count": 0,
        "entity_count": 0,
    }

    for path in fixture_paths:
        name = fixture_name(path)
        try:
            fixture = json.loads(path.read_text())
        except json.JSONDecodeError as exc:
            errors.append(f"{path.name}: invalid JSON: {exc}")
            continue

        if not isinstance(fixture, dict):
            errors.append(f"{path.name}: top-level value must be an object")
            continue

        expected_format = SPECIAL_FORMATS.get(
            name, f"whatwg-html-tokenizer-{name}/v1"
        )
        if fixture.get("format") != expected_format:
            errors.append(
                f"{path.name}: expected format {expected_format!r}, "
                f"got {fixture.get('format')!r}"
            )

        if name == "entities":
            errors.extend(check_entities_fixture(path, fixture, stats))
        elif name == "numeric-references":
            errors.extend(check_numeric_reference_fixture(path, fixture, stats))
        elif name == "input-stream":
            errors.extend(check_input_stream_fixture(path, fixture, stats))
        else:
            errors.extend(check_tokenizer_fixture(path, fixture, stats))

    return errors, stats


def check_entities_fixture(
    path: Path, fixture: dict[str, Any], stats: dict[str, int]
) -> list[str]:
    errors = check_descriptionless_source_fixture(path, fixture)
    entities = fixture.get("entities")
    if not isinstance(entities, list) or not entities:
        return [*errors, f"{path.name}: entities must be a non-empty list"]

    seen_names: set[str] = set()
    for index, entity in enumerate(entities):
        prefix = f"{path.name}: entities[{index}]"
        if not isinstance(entity, dict):
            errors.append(f"{prefix}: must be an object")
            continue

        name = entity.get("name")
        characters = entity.get("characters")
        codepoints = entity.get("codepoints")
        semicolon = entity.get("semicolon")

        if not isinstance(name, str) or not name.startswith("&"):
            errors.append(f"{prefix}: name must start with '&'")
        elif name in seen_names:
            errors.append(f"{prefix}: duplicate entity name {name!r}")
        else:
            seen_names.add(name)

        if not isinstance(characters, str) or characters == "":
            errors.append(f"{prefix}: characters must be a non-empty string")
        if not is_int_list(codepoints):
            errors.append(f"{prefix}: codepoints must be a list of integers")
        elif isinstance(characters, str) and [ord(char) for char in characters] != codepoints:
            errors.append(f"{prefix}: characters/codepoints mismatch")

        if not isinstance(semicolon, bool):
            errors.append(f"{prefix}: semicolon must be a boolean")
        elif isinstance(name, str) and semicolon != name.endswith(";"):
            errors.append(f"{prefix}: semicolon flag does not match name")

    stats["entity_count"] += len(entities)
    return errors


def check_numeric_reference_fixture(
    path: Path, fixture: dict[str, Any], stats: dict[str, int]
) -> list[str]:
    errors = check_described_fixture(path, fixture)
    cases = fixture.get("cases")
    if not isinstance(cases, list) or not cases:
        return [*errors, f"{path.name}: cases must be a non-empty list"]

    seen_values: set[int] = set()
    required_fields = {
        "characters",
        "codepoints",
        "decimal",
        "decimal_missing_semicolon",
        "diagnostics",
        "hex",
        "hex_missing_semicolon",
        "value",
    }
    for index, case in enumerate(cases):
        prefix = f"{path.name}: cases[{index}]"
        if not isinstance(case, dict):
            errors.append(f"{prefix}: must be an object")
            continue

        missing = required_fields - case.keys()
        if missing:
            errors.append(f"{prefix}: missing fields {', '.join(sorted(missing))}")

        value = case.get("value")
        if not isinstance(value, int):
            errors.append(f"{prefix}: value must be an integer")
        elif value in seen_values:
            errors.append(f"{prefix}: duplicate value {value}")
        else:
            seen_values.add(value)

        characters = case.get("characters")
        codepoints = case.get("codepoints")
        if not isinstance(characters, str):
            errors.append(f"{prefix}: characters must be a string")
        if not is_int_list(codepoints):
            errors.append(f"{prefix}: codepoints must be a list of integers")
        elif isinstance(characters, str) and [ord(char) for char in characters] != codepoints:
            errors.append(f"{prefix}: characters/codepoints mismatch")

        for field in (
            "decimal",
            "decimal_missing_semicolon",
            "hex",
            "hex_missing_semicolon",
        ):
            if not isinstance(case.get(field), str) or not case[field].startswith("&#"):
                errors.append(f"{prefix}: {field} must be an HTML numeric reference")

        if not is_str_list(case.get("diagnostics")):
            errors.append(f"{prefix}: diagnostics must be a list of strings")

    stats["case_count"] += len(cases)
    return errors


def check_input_stream_fixture(
    path: Path, fixture: dict[str, Any], stats: dict[str, int]
) -> list[str]:
    errors = check_described_fixture(path, fixture)
    cases = fixture.get("cases")
    if not isinstance(cases, list) or not cases:
        return [*errors, f"{path.name}: cases must be a non-empty list"]

    for field in ("newline_forms", "position_cases"):
        if not isinstance(fixture.get(field), list) or not fixture[field]:
            errors.append(f"{path.name}: {field} must be a non-empty list")

    seen_ids: set[str] = set()
    for index, case in enumerate(cases):
        prefix = f"{path.name}: cases[{index}]"
        if not isinstance(case, dict):
            errors.append(f"{prefix}: must be an object")
            continue
        errors.extend(check_case_id(case, prefix, seen_ids))
        for field in ("description", "input", "normalized"):
            if not isinstance(case.get(field), str):
                errors.append(f"{prefix}: {field} must be a string")

    stats["case_count"] += len(cases)
    return errors


def check_tokenizer_fixture(
    path: Path, fixture: dict[str, Any], stats: dict[str, int]
) -> list[str]:
    errors = check_described_fixture(path, fixture)
    cases = fixture.get("cases")
    if not isinstance(cases, list) or not cases:
        return [*errors, f"{path.name}: cases must be a non-empty list"]

    seen_ids: set[str] = set()
    for index, case in enumerate(cases):
        prefix = f"{path.name}: cases[{index}]"
        if not isinstance(case, dict):
            errors.append(f"{prefix}: must be an object")
            continue

        errors.extend(check_case_id(case, prefix, seen_ids))
        missing = REQUIRED_CASE_FIELDS - case.keys()
        if missing:
            errors.append(f"{prefix}: missing fields {', '.join(sorted(missing))}")

        if not isinstance(case.get("description"), str):
            errors.append(f"{prefix}: description must be a string")
        if not isinstance(case.get("input"), str):
            errors.append(f"{prefix}: input must be a string")
        if "tokens" in case and not is_str_list(case.get("tokens")):
            errors.append(f"{prefix}: tokens must be a list of strings")
        if "diagnostics" in case and not is_str_list(case.get("diagnostics")):
            errors.append(f"{prefix}: diagnostics must be a list of strings")
        if "split_points" in case and not is_int_list(case.get("split_points")):
            errors.append(f"{prefix}: split_points must be a list of integers")

    stats["case_count"] += len(cases)
    return errors


def check_described_fixture(path: Path, fixture: dict[str, Any]) -> list[str]:
    if not isinstance(fixture.get("description"), str) or not fixture["description"]:
        return [f"{path.name}: description must be a non-empty string"]
    return []


def check_descriptionless_source_fixture(path: Path, fixture: dict[str, Any]) -> list[str]:
    source = fixture.get("source")
    if not isinstance(source, str) or not source:
        return [f"{path.name}: source must be a non-empty string"]
    return []


def check_case_id(case: dict[str, Any], prefix: str, seen_ids: set[str]) -> list[str]:
    errors: list[str] = []
    case_id = case.get("id")
    if not isinstance(case_id, str) or not case_id:
        errors.append(f"{prefix}: id must be a non-empty string")
    elif case_id in seen_ids:
        errors.append(f"{prefix}: duplicate id {case_id!r}")
    else:
        seen_ids.add(case_id)
    return errors


def fixture_name(path: Path) -> str:
    return path.stem.removeprefix("whatwg-")


def generator_fixture_name(path: Path) -> str:
    name = path.stem.removeprefix("generate_whatwg_").removesuffix("_fixture")
    return name.replace("_", "-")


def is_int_list(value: Any) -> bool:
    return isinstance(value, list) and all(isinstance(item, int) for item in value)


def is_str_list(value: Any) -> bool:
    return isinstance(value, list) and all(isinstance(item, str) for item in value)


if __name__ == "__main__":
    raise SystemExit(main())
