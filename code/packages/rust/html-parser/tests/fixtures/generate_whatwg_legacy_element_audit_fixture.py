#!/usr/bin/env python3

"""Generate Venture's focused WHATWG legacy/edge element audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-legacy-element-audit.json"

CASE_FAMILIES = (
    {
        "source_file": "isindex.dat",
        "axis": "legacy-isindex",
        "reason": "obsolete isindex insertion and form-control replacement recovery",
    },
    {
        "source_file": "menuitem-element.dat",
        "axis": "obsolete-menuitem",
        "reason": "obsolete menuitem parsing through body, list, table, and select contexts",
    },
    {
        "source_file": "main-element.dat",
        "axis": "main-element-boundary",
        "reason": "main element boundaries around paragraphs and nested main starts",
    },
    {
        "source_file": "search-element.dat",
        "axis": "search-element-boundary",
        "reason": "search element boundaries around paragraphs and nested search starts",
    },
    {
        "source_file": "pending-spec-changes.dat",
        "axis": "pending-spec-boundary",
        "reason": "pending WHATWG tree-construction edge cases in the checked corpus",
    },
    {
        "source_file": "pending-spec-changes-plain-text-unsafe.dat",
        "axis": "pending-spec-boundary",
        "reason": "pending WHATWG tree-construction edge cases in the checked corpus",
    },
    {
        "source_file": "tricky01.dat",
        "axis": "tricky-parser-recovery",
        "reason": "legacy tricky parser recovery cases retained by html5lib",
    },
    {
        "source_file": "namespace-sensitivity.dat",
        "axis": "namespace-sensitivity",
        "reason": "namespace-sensitive integration-point recovery in the parser DOM",
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
        description="Generate the focused WHATWG legacy/edge element audit fixture."
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
    selected = []
    seen = set()

    for source in sources:
        family = family_for_source(source)
        if family is None:
            continue
        if source in seen:
            raise SystemExit(f"duplicate selected source: {source}")
        seen.add(source)
        selected.append(
            {
                "id": stable_case_id(source),
                "source": source,
                "axis": family["axis"],
                "reason": family["reason"],
            }
        )

    if not selected:
        raise SystemExit("no legacy/edge element audit cases matched configured families")

    axes = list(dict.fromkeys(family["axis"] for family in CASE_FAMILIES))
    counts_by_axis = {
        axis: sum(1 for case in selected if case["axis"] == axis) for axis in axes
    }
    missing_axes = [axis for axis, count in counts_by_axis.items() if count == 0]
    if missing_axes:
        raise SystemExit(f"missing selected cases for axes: {', '.join(missing_axes)}")

    return {
        "format": "whatwg-html-legacy-element-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction "
            "cases that stress legacy, obsolete, pending-spec, tricky, and "
            "namespace-sensitive element recovery."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def family_for_source(source: str) -> dict[str, str] | None:
    source_file = source.split(":", 1)[0]
    for family in CASE_FAMILIES:
        if source_file == family["source_file"]:
            return family
    return None


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
