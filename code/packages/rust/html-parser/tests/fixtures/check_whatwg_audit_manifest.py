#!/usr/bin/env python3

"""Check metadata and generator coverage for focused WHATWG parser audits."""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
LEXER_FIXTURE_DIR = FIXTURE_DIR.parents[2] / "html-lexer" / "tests" / "fixtures"
AUDIT_GLOB = "whatwg-*-audit.json"
SOURCE_FIXTURE = "html5lib-tree-construction-smoke.dat"


def main() -> int:
    parse_args()
    audit_paths = sorted(FIXTURE_DIR.glob(AUDIT_GLOB))
    generator_paths = sorted(FIXTURE_DIR.glob("generate_whatwg_*_audit_fixture.py"))

    errors: list[str] = []
    errors.extend(check_generator_pairs(audit_paths, generator_paths))
    errors.extend(check_manifest_wiring(generator_paths))
    errors.extend(check_audit_metadata(audit_paths))

    print("WHATWG parser audit manifest")
    print(f"audit files: {len(audit_paths)}")
    print(f"generators:  {len(generator_paths)}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check that focused WHATWG parser audit fixtures keep matching "
            "generators, manifest wiring, and JSON metadata."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for the generated fixture manifest.",
    )
    return parser.parse_args()


def check_generator_pairs(
    audit_paths: list[Path], generator_paths: list[Path]
) -> list[str]:
    errors: list[str] = []
    audit_names = {audit_name(path) for path in audit_paths}
    generator_names = {generator_name(path) for path in generator_paths}

    missing_generators = sorted(audit_names - generator_names)
    stale_generators = sorted(generator_names - audit_names)
    if missing_generators:
        errors.append(
            "audit fixtures without matching generators:\n"
            + "\n".join(f"  {name}" for name in missing_generators)
        )
    if stale_generators:
        errors.append(
            "audit generators without matching fixtures:\n"
            + "\n".join(f"  {name}" for name in stale_generators)
        )
    return errors


def check_manifest_wiring(generator_paths: list[Path]) -> list[str]:
    manifest_path = LEXER_FIXTURE_DIR / "check_generated_html_fixtures.py"
    manifest = manifest_path.read_text()

    missing = [
        path.name
        for path in generator_paths
        if f'"{path.name}"' not in manifest
    ]
    if "check_whatwg_audit_manifest.py" not in manifest:
        missing.append("check_whatwg_audit_manifest.py")

    if not missing:
        return []
    return [
        "parser audit checks missing from generated fixture manifest:\n"
        + "\n".join(f"  {name}" for name in sorted(missing))
    ]


def check_audit_metadata(audit_paths: list[Path]) -> list[str]:
    errors: list[str] = []
    for audit_path in audit_paths:
        audit = json.loads(audit_path.read_text())
        name = audit_name(audit_path)
        cases = audit.get("cases")
        if not isinstance(cases, list) or not cases:
            errors.append(f"{audit_path.name} has no non-empty cases list")
            continue

        expected_format = f"whatwg-html-{name}-audit/v1"
        if audit.get("format") != expected_format:
            errors.append(
                f"{audit_path.name} format is {audit.get('format')!r}; "
                f"expected {expected_format!r}"
            )
        if audit.get("source_fixture") != SOURCE_FIXTURE:
            errors.append(
                f"{audit_path.name} source_fixture is {audit.get('source_fixture')!r}; "
                f"expected {SOURCE_FIXTURE!r}"
            )
        if audit.get("case_count") != len(cases):
            errors.append(
                f"{audit_path.name} case_count is {audit.get('case_count')!r}; "
                f"expected {len(cases)}"
            )

        axes = Counter()
        ids: set[str] = set()
        duplicate_ids: list[str] = []
        for index, case in enumerate(cases):
            prefix = f"{audit_path.name} case {index}"
            for field in ("id", "source", "axis", "reason"):
                if not isinstance(case.get(field), str) or not case[field]:
                    errors.append(f"{prefix} has invalid {field!r}: {case.get(field)!r}")
            axis = case.get("axis")
            if isinstance(axis, str):
                axes[axis] += 1
            case_id = case.get("id")
            if isinstance(case_id, str):
                if case_id in ids:
                    duplicate_ids.append(case_id)
                ids.add(case_id)

        if duplicate_ids:
            errors.append(
                f"{audit_path.name} duplicate case ids:\n"
                + "\n".join(f"  {case_id}" for case_id in sorted(set(duplicate_ids)))
            )
        if audit.get("counts_by_axis") != dict(sorted(axes.items())):
            errors.append(
                f"{audit_path.name} counts_by_axis does not match case axes"
            )

    return errors


def audit_name(path: Path) -> str:
    return path.name.removeprefix("whatwg-").removesuffix("-audit.json")


def generator_name(path: Path) -> str:
    return (
        path.name.removeprefix("generate_whatwg_")
        .removesuffix("_audit_fixture.py")
        .replace("_", "-")
    )


if __name__ == "__main__":
    raise SystemExit(main())
