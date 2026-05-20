#!/usr/bin/env python3

"""Generate Venture's focused WHATWG fragment-context audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-fragment-context-audit.json"

TABLE_CONTEXTS = {
    "caption",
    "colgroup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
}
TEXT_MODE_CONTEXTS = {
    "iframe",
    "noembed",
    "noframes",
    "noscript",
    "plaintext",
    "script",
    "style",
    "textarea",
    "title",
    "xmp",
}
SELECT_CONTEXTS = {"optgroup", "option", "select"}
SHELL_CONTEXTS = {"body", "frameset", "head", "html"}

FOREIGN_CONTEXT = re.compile(r"^(?:math|svg)(?:\s|$)", re.I)
FOREIGN_MARKUP = re.compile(r"</?(?:svg|math|foreignObject|desc)(?:\s|/|>)", re.I)
TEMPLATE_MARKUP = re.compile(r"</?template(?:\s|/|>)", re.I)
BLOCK_MARKUP = re.compile(
    r"</?(?:address|article|aside|blockquote|center|details|dialog|dir|div|dl|"
    r"fieldset|figcaption|figure|footer|header|hgroup|main|menu|nav|ol|p|section|"
    r"summary|ul|h[1-6]|li|dt|dd)(?:\s|/|>)",
    re.I,
)

CASE_AXES = (
    {
        "axis": "fragment-table-context",
        "reason": "fragment parser context inside table, row, column, caption, and cell modes",
    },
    {
        "axis": "fragment-basic-context",
        "reason": "fragment parser context with ordinary phrasing and element boundaries",
    },
    {
        "axis": "fragment-block-context",
        "reason": "fragment parser context around block and implied-end-tag boundaries",
    },
    {
        "axis": "fragment-shell-context",
        "reason": "fragment parser context seeded from html, head, body, and frameset elements",
    },
    {
        "axis": "fragment-foreign-context",
        "reason": "fragment parser context in SVG and MathML integration modes",
    },
    {
        "axis": "fragment-text-mode-context",
        "reason": "fragment parser context for RCDATA, RAWTEXT, PLAINTEXT, and legacy text modes",
    },
    {
        "axis": "fragment-select-context",
        "reason": "fragment parser context in select, option, and optgroup insertion modes",
    },
    {
        "axis": "fragment-template-context",
        "reason": "fragment parser context seeded from template insertion modes",
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
        description="Generate the focused WHATWG fragment-context audit fixture."
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
        if fragment_context is not None:
            cases.append(
                SmokeCase(
                    source=source,
                    data=case_data,
                    fragment_context=fragment_context,
                )
            )

    if not cases:
        raise SystemExit(f"{path} does not contain any fragment-context audit cases")
    return cases


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
                "context": case.fragment_context,
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
        "format": "whatwg-html-fragment-context-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction "
            "cases that stress fragment parsing contexts across table, shell, "
            "block, foreign-content, text-mode, select/list, "
            "template, and ordinary phrasing modes."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def axis_for_case(case: SmokeCase) -> str:
    data = case.data
    context = case.fragment_context or ""
    context_head = context.split()[0].lower()

    if context_head in TABLE_CONTEXTS:
        return "fragment-table-context"
    if context_head in TEXT_MODE_CONTEXTS:
        return "fragment-text-mode-context"
    if context_head in SELECT_CONTEXTS:
        return "fragment-select-context"
    if FOREIGN_CONTEXT.search(context) or FOREIGN_MARKUP.search(data):
        return "fragment-foreign-context"
    if context_head == "template" or TEMPLATE_MARKUP.search(data):
        return "fragment-template-context"
    if context_head in SHELL_CONTEXTS:
        return "fragment-shell-context"
    if BLOCK_MARKUP.search(data):
        return "fragment-block-context"
    return "fragment-basic-context"


def reason_for_axis(axis: str) -> str:
    for axis_info in CASE_AXES:
        if axis_info["axis"] == axis:
            return axis_info["reason"]
    raise SystemExit(f"unknown fragment-context audit axis: {axis}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
