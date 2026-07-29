#!/usr/bin/env python3

"""Generate Venture's focused WHATWG processing-instruction audit fixture."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
LEXER_FIXTURE_DIR = FIXTURE_DIR.parents[2] / "html-lexer" / "tests" / "fixtures"
sys.path.insert(0, str(LEXER_FIXTURE_DIR))

from generated_fixture_io import write_fixture_json

DEFAULT_INPUT = FIXTURE_DIR / "html5lib-tree-construction-smoke.dat"
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-processing-instruction-audit.json"
SOURCE_PREFIX = "processing-instructions.dat:"

CASE_GROUPS = (
    (
        range(1, 66),
        "valid-target-and-data",
        "valid processing-instruction targets and normalized data",
    ),
    (
        range(66, 101),
        "invalid-target-recovery",
        "reserved or invalid targets recovered as bogus comments",
    ),
    (
        range(101, 108),
        "eof-recovery",
        "incomplete processing instructions at end of input",
    ),
    (
        range(108, 125),
        "insertion-context",
        "processing instructions across tree-builder insertion contexts",
    ),
)


def main() -> int:
    args = parse_args()
    input_path = Path(args.input).expanduser().resolve()
    output_path = Path(args.output).expanduser().resolve()
    sources = parse_sources(input_path)
    fixture = build_fixture(sources)
    return write_fixture_json(output_path, fixture, check=args.check, ensure_ascii=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate the focused WHATWG processing-instruction audit fixture."
    )
    parser.add_argument(
        "--input",
        default=str(DEFAULT_INPUT),
        help="html5lib tree-construction smoke fixture to index.",
    )
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="Fixture path to write; defaults beside this script.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Exit non-zero if the checked-in fixture differs from generated output.",
    )
    return parser.parse_args()


def parse_sources(path: Path) -> list[str]:
    sources = [
        line.removeprefix("#source ").strip()
        for line in path.read_text(errors="replace").splitlines()
        if line.startswith(f"#source {SOURCE_PREFIX}")
    ]
    if not sources:
        raise SystemExit(f"{path} does not contain processing-instruction cases")
    return sources


def build_fixture(sources: list[str]) -> dict[str, object]:
    selected = []
    seen_numbers = set()
    counts_by_axis = {axis: 0 for _, axis, _ in CASE_GROUPS}

    for source in sources:
        case_number = int(source.removeprefix(SOURCE_PREFIX))
        if case_number in seen_numbers:
            raise SystemExit(f"duplicate processing-instruction case: {case_number}")
        seen_numbers.add(case_number)
        axis, reason = metadata_for_case(case_number)
        counts_by_axis[axis] += 1
        selected.append(
            {
                "id": stable_case_id(source),
                "source": source,
                "axis": axis,
                "reason": reason,
            }
        )

    expected_numbers = set(range(1, 125))
    if seen_numbers != expected_numbers:
        missing = sorted(expected_numbers - seen_numbers)
        unexpected = sorted(seen_numbers - expected_numbers)
        raise SystemExit(
            f"processing-instruction case mismatch; missing={missing}, unexpected={unexpected}"
        )

    return {
        "format": "whatwg-html-processing-instruction-audit/v1",
        "description": (
            "Focused parser audit over the current WPT processing-instruction "
            "tree-construction corpus, including target validation, data "
            "normalization, EOF recovery, and insertion contexts."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def metadata_for_case(case_number: int) -> tuple[str, str]:
    for case_numbers, axis, reason in CASE_GROUPS:
        if case_number in case_numbers:
            return axis, reason
    raise SystemExit(f"unclassified processing-instruction case: {case_number}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
