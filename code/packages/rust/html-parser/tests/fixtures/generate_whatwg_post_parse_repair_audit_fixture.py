#!/usr/bin/env python3

"""Generate Venture's focused WHATWG post-parse repair audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-post-parse-repair-audit.json"

CASE_GROUPS = (
    {
        "axis": "adoption-table-foster-parenting",
        "reason": (
            "remaining repair evidence for adoption-agency formatting inside "
            "table foster-parenting recovery"
        ),
        "sources": ("adoption01.dat:6",),
    },
    {
        "axis": "fostered-nobr-cell-continuation",
        "reason": (
            "remaining finish-time recovery for fostered nobr continuation "
            "nodes that html5lib keeps inside the table cell"
        ),
        "sources": (
            "tests26.dat:4",
            "tests26.dat:1251",
        ),
    },
    {
        "axis": "tricky-center-table-void-recovery",
        "reason": (
            "remaining repair evidence for center, font, and void-element "
            "recovery through table cell insertion modes"
        ),
        "sources": ("tricky01.dat:6",),
    },
    {
        "axis": "tricky-paragraph-rowgroup-recovery",
        "reason": (
            "remaining repair evidence for paragraph and anchor recovery "
            "crossing table row-group insertion modes"
        ),
        "sources": ("tricky01.dat:7",),
    },
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
        description="Generate the focused WHATWG post-parse repair audit fixture."
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
    sources: list[str] = []
    for line in path.read_text(errors="replace").splitlines():
        if line.startswith("#source "):
            sources.append(line.removeprefix("#source ").strip())
    if not sources:
        raise SystemExit(f"{path} does not contain any #source markers")
    return sources


def build_fixture(sources: list[str]) -> dict[str, object]:
    case_by_source = configured_cases_by_source()
    selected = []
    seen = set()

    for source in sources:
        case = case_by_source.get(source)
        if case is None:
            continue
        if source in seen:
            raise SystemExit(f"duplicate selected source: {source}")
        seen.add(source)
        selected.append(
            {
                "id": stable_case_id(source),
                "source": source,
                "axis": case["axis"],
                "reason": case["reason"],
            }
        )

    missing_sources = [source for source in case_by_source if source not in seen]
    if missing_sources:
        raise SystemExit(
            "missing configured post-parse repair sources: "
            + ", ".join(missing_sources)
        )

    axes = [case_group["axis"] for case_group in CASE_GROUPS]
    counts_by_axis = {
        axis: sum(1 for case in selected if case["axis"] == axis) for axis in axes
    }
    missing_axes = [axis for axis, count in counts_by_axis.items() if count == 0]
    if missing_axes:
        raise SystemExit(f"missing selected cases for axes: {', '.join(missing_axes)}")

    return {
        "format": "whatwg-html-post-parse-repair-audit/v1",
        "description": (
            "Focused parser audit over the remaining html5lib tree-construction "
            "cases that justify post-parse repair shims or adjacent repair "
            "coverage."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def configured_cases_by_source() -> dict[str, dict[str, str]]:
    cases: dict[str, dict[str, str]] = {}
    for case_group in CASE_GROUPS:
        for source in case_group["sources"]:
            if source in cases:
                raise SystemExit(f"duplicate configured source: {source}")
            cases[source] = {
                "axis": case_group["axis"],
                "reason": case_group["reason"],
            }
    return cases


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
