#!/usr/bin/env python3

"""Generate Venture's focused WHATWG tree-insertion audit fixture.

The parser already carries a large html5lib tree-construction smoke corpus. This
fixture indexes the high-signal insertion-mode families inside that corpus so CI
can report parser regressions against adoption-agency, table/foster-parenting,
template, foreign-content fragment, and fragment-shell recovery separately from
the catch-all smoke test.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
DEFAULT_INPUT = FIXTURE_DIR / "html5lib-tree-construction-smoke.dat"
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-tree-insertion-audit.json"

CASE_FAMILIES = (
    {
        "prefix": "adoption",
        "axis": "adoption-agency",
        "reason": "misnested formatting elements and adoption-agency recovery",
    },
    {
        "prefix": "tables01.dat",
        "axis": "table-insertion",
        "reason": "table insertion modes, implied rows/cells, and foster parenting",
    },
    {
        "prefix": "template.dat",
        "axis": "template-insertion",
        "reason": "template contents and template-context insertion modes",
    },
    {
        "prefix": "foreign-fragment.dat",
        "axis": "foreign-fragment",
        "reason": "foreign-content fragment contexts and HTML integration recovery",
    },
    {
        "prefix": "tests_innerHTML_1.dat",
        "axis": "html-fragment",
        "reason": "innerHTML fragment shell recovery across HTML contexts",
    },
)


def main() -> int:
    args = parse_args()
    input_path = Path(args.input).expanduser().resolve()
    output_path = Path(args.output).expanduser().resolve()

    sources = parse_sources(input_path)
    fixture = build_fixture(sources)
    text = json.dumps(fixture, indent=2, sort_keys=True) + "\n"

    if args.check:
        existing = output_path.read_text()
        if existing != text:
            raise SystemExit(f"{output_path} is stale; regenerate it")
        return 0

    output_path.write_text(text)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate the focused WHATWG tree-insertion audit fixture."
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
    for line in path.read_text().splitlines():
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
        raise SystemExit("no tree-insertion audit cases matched the configured families")

    counts_by_axis = {
        family["axis"]: sum(1 for case in selected if case["axis"] == family["axis"])
        for family in CASE_FAMILIES
    }
    missing_axes = [axis for axis, count in counts_by_axis.items() if count == 0]
    if missing_axes:
        raise SystemExit(f"missing selected cases for axes: {', '.join(missing_axes)}")

    return {
        "format": "whatwg-html-tree-insertion-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction "
            "families that stress insertion-mode recovery."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def family_for_source(source: str) -> dict[str, str] | None:
    for family in CASE_FAMILIES:
        prefix = family["prefix"]
        if prefix.endswith(".dat"):
            if source.startswith(f"{prefix}:"):
                return family
        elif source.startswith(prefix):
            return family
    return None


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
