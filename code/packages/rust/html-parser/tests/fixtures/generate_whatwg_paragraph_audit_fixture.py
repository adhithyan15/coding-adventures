#!/usr/bin/env python3

"""Generate Venture's focused WHATWG paragraph audit fixture."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
LEXER_FIXTURE_DIR = FIXTURE_DIR.parents[2] / "html-lexer" / "tests" / "fixtures"
sys.path.insert(0, str(LEXER_FIXTURE_DIR))

from generated_fixture_io import write_fixture_json

DEFAULT_INPUT = FIXTURE_DIR / "html5lib-tree-construction-smoke.dat"
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-paragraph-audit.json"

PARAGRAPH_MARKUP = re.compile(r"</?p(?:\s|/|>)", re.I)
BLOCK_MARKUP = re.compile(
    r"<(?:address|article|aside|blockquote|center|details|dialog|dir|div|dl|fieldset|figcaption|figure|footer|header|hgroup|main|menu|nav|ol|section|ul)(?:\s|/|>)",
    re.I,
)
HEADING_MARKUP = re.compile(r"<h[1-6](?:\s|/|>)", re.I)
TABLE_MARKUP = re.compile(
    r"<(?:table|tbody|tfoot|thead|tr|td|th|caption)(?:\s|/|>)",
    re.I,
)
FORM_MARKUP = re.compile(r"<(?:form|button|input|select|textarea)(?:\s|/|>)", re.I)
TEXT_MODE_MARKUP = re.compile(
    r"<(?:pre|listing|plaintext|textarea|script|style|title)(?:\s|/|>)",
    re.I,
)
FORMATTING_MARKUP = re.compile(
    r"</?(?:a|b|big|code|em|font|i|nobr|s|small|strike|strong|tt|u)(?:\s|/|>)",
    re.I,
)
SPECIAL_PARAGRAPH_END_MARKUP = re.compile(r"</(?:p|br)(?:\s|/|>)", re.I)

CASE_AXES = (
    {
        "axis": "paragraph-form-boundary",
        "reason": "paragraph boundaries around form controls and interactive descendants",
    },
    {
        "axis": "paragraph-formatting-boundary",
        "reason": "paragraph boundaries with active formatting reconstruction",
    },
    {
        "axis": "paragraph-block-boundary",
        "reason": "paragraph implied end tags before block-level boundaries",
    },
    {
        "axis": "paragraph-basic-boundary",
        "reason": "standalone paragraph starts, ends, and omitted-end-tag recovery",
    },
    {
        "axis": "paragraph-table-boundary",
        "reason": "paragraph boundaries around table insertion and foster parenting modes",
    },
    {
        "axis": "paragraph-text-mode-boundary",
        "reason": "paragraph boundaries around pre, listing, plaintext, and text-mode elements",
    },
    {
        "axis": "paragraph-special-end-tag",
        "reason": "special paragraph and br end-tag recovery paths",
    },
    {
        "axis": "paragraph-heading-boundary",
        "reason": "paragraph implied end tags before heading starts",
    },
    {
        "axis": "paragraph-fragment-context",
        "reason": "paragraph handling in html5lib fragment contexts",
    },
)


@dataclass(frozen=True)
class SmokeCase:
    source: str
    data: str
    fragment_context: str | None


def main() -> int:
    args = parse_args()
    input_path = Path(args.input).expanduser().resolve()
    output_path = Path(args.output).expanduser().resolve()

    cases = parse_cases(input_path)
    fixture = build_fixture(cases)
    return write_fixture_json(output_path, fixture, check=args.check, ensure_ascii=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate the focused WHATWG paragraph audit fixture."
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


def parse_cases(path: Path) -> list[SmokeCase]:
    cases: list[SmokeCase] = []
    lines = path.read_text(errors="replace").splitlines()
    source = ""
    index = 0

    while index < len(lines):
        line = lines[index]
        index += 1
        if not line:
            continue
        if line.startswith("#source "):
            source = line.removeprefix("#source ").strip()
            continue
        if line != "#data":
            raise SystemExit(f"expected #data after {source}, got {line!r}")

        data: list[str] = []
        while index < len(lines):
            data_line = lines[index]
            index += 1
            if data_line == "#errors":
                break
            data.append(data_line)

        fragment_context = None
        while index < len(lines):
            metadata_line = lines[index]
            index += 1
            if metadata_line == "#document":
                break
            if metadata_line == "#document-fragment":
                if index >= len(lines):
                    raise SystemExit(f"missing fragment context after {source}")
                fragment_context = lines[index]
                index += 1

        while index < len(lines):
            document_line = lines[index]
            if document_line == "#data" or document_line.startswith("#source "):
                break
            index += 1

        case_data = "\n".join(data)
        if is_paragraph_case(case_data):
            cases.append(
                SmokeCase(
                    source=source,
                    data=case_data,
                    fragment_context=fragment_context,
                )
            )

    if not cases:
        raise SystemExit(f"{path} does not contain any paragraph audit cases")
    return cases


def is_paragraph_case(data: str) -> bool:
    return PARAGRAPH_MARKUP.search(data) is not None


def build_fixture(cases: list[SmokeCase]) -> dict[str, object]:
    selected = []
    seen = set()

    for case in cases:
        if case.source in seen:
            raise SystemExit(f"duplicate selected source: {case.source}")
        seen.add(case.source)
        axis = axis_for_case(case)
        selected.append(
            {
                "id": stable_case_id(case.source),
                "source": case.source,
                "axis": axis,
                "reason": reason_for_axis(axis),
            }
        )

    counts_by_axis = {
        axis["axis"]: sum(1 for case in selected if case["axis"] == axis["axis"])
        for axis in CASE_AXES
    }
    missing_axes = [axis for axis, count in counts_by_axis.items() if count == 0]
    if missing_axes:
        raise SystemExit(f"missing selected cases for axes: {', '.join(missing_axes)}")

    return {
        "format": "whatwg-html-paragraph-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction cases "
            "that stress paragraph implied end tags, formatting reconstruction, "
            "table/foster-parenting boundaries, form controls, text modes, "
            "headings, special end-tag recovery, and fragment contexts."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def axis_for_case(case: SmokeCase) -> str:
    data = case.data

    if case.fragment_context is not None:
        return "paragraph-fragment-context"
    if TABLE_MARKUP.search(data):
        return "paragraph-table-boundary"
    if TEXT_MODE_MARKUP.search(data):
        return "paragraph-text-mode-boundary"
    if FORM_MARKUP.search(data):
        return "paragraph-form-boundary"
    if FORMATTING_MARKUP.search(data):
        return "paragraph-formatting-boundary"
    if HEADING_MARKUP.search(data):
        return "paragraph-heading-boundary"
    if BLOCK_MARKUP.search(data):
        return "paragraph-block-boundary"
    if SPECIAL_PARAGRAPH_END_MARKUP.search(data):
        return "paragraph-special-end-tag"
    return "paragraph-basic-boundary"


def reason_for_axis(axis: str) -> str:
    for axis_info in CASE_AXES:
        if axis_info["axis"] == axis:
            return axis_info["reason"]
    raise SystemExit(f"unknown paragraph audit axis: {axis}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
