#!/usr/bin/env python3

"""Generate Venture's focused WHATWG miscellaneous recovery audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-misc-recovery-audit.json"

CASE_GROUPS = (
    {
        "axis": "xml-pi-looking-markup",
        "reason": "XML declaration and processing-instruction-looking input recovery",
        "sources": (
            "comments01.dat:79",
            "comments01.dat:80",
            "comments01.dat:81",
            "html5test-com.dat:12",
            "tests1.dat:40",
            "tests1.dat:41",
            "tests1.dat:44",
            "tests1.dat:47",
            "tests1.dat:605",
            "tests1.dat:606",
            "tests1.dat:609",
            "tests1.dat:612",
        ),
    },
    {
        "axis": "bogus-comment-and-cdata",
        "reason": "bogus comment, malformed declaration, and CDATA-as-text recovery",
        "sources": (
            "html5test-com.dat:283",
            "html5test-com.dat:284",
            "tests1.dat:42",
            "tests1.dat:43",
            "tests1.dat:45",
            "tests1.dat:46",
            "tests1.dat:48",
            "tests1.dat:49",
            "tests1.dat:607",
            "tests1.dat:608",
            "tests1.dat:610",
            "tests1.dat:611",
            "tests1.dat:613",
            "tests1.dat:614",
            "tests2.dat:35",
            "tests2.dat:42",
            "tests2.dat:43",
            "tests2.dat:44",
            "tests2.dat:60",
            "tests2.dat:62",
        ),
    },
    {
        "axis": "text-whitespace-shell",
        "reason": "plain text, whitespace, NUL, and minimal document-shell recovery",
        "sources": (
            "html5test-com.dat:276",
            "plain-text-unsafe.dat:356",
            "tests1.dat:1",
            "tests1.dat:566",
            "tests1.dat:62",
            "tests1.dat:627",
            "tests2.dat:1",
            "tests2.dat:46",
            "tests2.dat:50",
            "webkit01.dat:1",
        ),
    },
    {
        "axis": "malformed-tag-open",
        "reason": "incomplete tag-open, bogus end-tag, long attribute, and slash recovery",
        "sources": (
            "tests1.dat:36",
            "tests1.dat:37",
            "tests1.dat:38",
            "tests1.dat:39",
            "tests1.dat:77",
            "tests1.dat:601",
            "tests1.dat:602",
            "tests1.dat:603",
            "tests1.dat:604",
            "tests1.dat:642",
            "webkit01.dat:4",
            "webkit01.dat:11",
            "webkit01.dat:14",
            "webkit02.dat:1",
        ),
    },
    {
        "axis": "legacy-compat-elements",
        "reason": "obsolete and compatibility element recovery outside larger families",
        "sources": (
            "tests1.dat:83",
            "tests1.dat:648",
            "tests19.dat:8",
            "tests19.dat:1021",
            "tests19.dat:89",
            "tests19.dat:1102",
            "tests25.dat:15",
            "tests25.dat:16",
            "tests25.dat:17",
            "tests25.dat:1236",
            "tests25.dat:1237",
            "tests25.dat:1238",
            "webkit02.dat:10",
        ),
    },
    {
        "axis": "custom-element-recovery",
        "reason": "unknown/custom element nesting, attributes, and stray end-tag recovery",
        "sources": (
            "webkit01.dat:8",
            "webkit01.dat:9",
            "webkit01.dat:10",
        ),
    },
    {
        "axis": "duplicate-doctype-recovery",
        "reason": "duplicate doctype and doctype-looking declaration recovery",
        "sources": (
            "domjs-unsafe.dat:147",
            "tests2.dat:45",
        ),
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
        description="Generate the focused WHATWG miscellaneous recovery audit fixture."
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
            "missing configured miscellaneous recovery sources: "
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
        "format": "whatwg-html-misc-recovery-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction "
            "cases that stress miscellaneous recovery around XML/PI-looking "
            "markup, bogus comments, CDATA-as-text, malformed tag opens, "
            "plain text, duplicate doctypes, unknown/custom elements, and "
            "legacy compatibility elements."
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
