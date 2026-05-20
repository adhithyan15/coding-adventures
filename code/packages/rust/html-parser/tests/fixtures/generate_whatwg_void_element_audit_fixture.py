#!/usr/bin/env python3

"""Generate Venture's focused WHATWG void-element audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-void-element-audit.json"

VOID_ELEMENTS = (
    "area",
    "base",
    "basefont",
    "bgsound",
    "br",
    "col",
    "embed",
    "frame",
    "hr",
    "img",
    "input",
    "link",
    "meta",
    "param",
    "source",
    "track",
    "wbr",
)
VOID_MARKUP = re.compile(
    r"</?(?:" + "|".join(VOID_ELEMENTS) + r")(?:\s|/|>)",
    re.I,
)
VOID_START = re.compile(
    r"<(?:" + "|".join(VOID_ELEMENTS) + r")(?:\s|/|>)",
    re.I,
)
VOID_END = re.compile(
    r"</(?:" + "|".join(VOID_ELEMENTS) + r")(?:\s|/|>)",
    re.I,
)
METADATA_VOID = re.compile(r"<(?:base|basefont|bgsound|link|meta)(?:\s|/|>)", re.I)
BODY_VOID = re.compile(r"<(?:area|br|embed|hr|img|input|param|source|track|wbr)(?:\s|/|>)", re.I)
TABLE_CONTEXT = re.compile(
    r"<(?:table|tbody|tfoot|thead|tr|td|th|caption|colgroup|col)(?:\s|/|>)",
    re.I,
)
SELECT_CONTEXT = re.compile(r"<(?:select|option|optgroup)(?:\s|/|>)", re.I)
FOREIGN_CONTEXT = re.compile(r"<(?:svg|math|foreignobject)(?:\s|/|>)", re.I)

CASE_AXES = (
    {
        "axis": "void-in-table",
        "reason": "void elements and void-like end tags through table insertion modes",
    },
    {
        "axis": "metadata-void-elements",
        "reason": "head metadata void elements in head, body, noscript, and templates",
    },
    {
        "axis": "body-void-elements",
        "reason": "body void elements such as area, br, embed, hr, img, input, and wbr",
    },
    {
        "axis": "stray-void-end-tags",
        "reason": "stray end tags for void elements and legacy void-like tags",
    },
    {
        "axis": "void-foreign-boundary",
        "reason": "void elements near SVG, MathML, and HTML integration points",
    },
    {
        "axis": "void-in-select",
        "reason": "void elements while select, option, and optgroup modes are active",
    },
    {
        "axis": "void-fragment-context",
        "reason": "void elements in html5lib fragment contexts",
    },
    {
        "axis": "legacy-void-elements",
        "reason": "legacy frame and other void elements outside specialized contexts",
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
        description="Generate the focused WHATWG void-element audit fixture."
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
        if is_void_case(case_data):
            cases.append(
                SmokeCase(
                    source=source,
                    data=case_data,
                    fragment_context=fragment_context,
                )
            )

    if not cases:
        raise SystemExit(f"{path} does not contain any void-element audit cases")
    return cases


def is_void_case(data: str) -> bool:
    return VOID_MARKUP.search(data) is not None


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
        "format": "whatwg-html-void-element-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction cases "
            "that stress void element insertion, stray void end tags, table/select "
            "contexts, fragment contexts, foreign-content boundaries, and legacy "
            "void-like elements."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def axis_for_case(case: SmokeCase) -> str:
    data = case.data.lower()

    if case.fragment_context is not None:
        return "void-fragment-context"
    if TABLE_CONTEXT.search(data) is not None:
        return "void-in-table"
    if SELECT_CONTEXT.search(data) is not None:
        return "void-in-select"
    if FOREIGN_CONTEXT.search(data) is not None:
        return "void-foreign-boundary"
    if VOID_END.search(data) is not None and VOID_START.search(data) is None:
        return "stray-void-end-tags"
    if METADATA_VOID.search(data) is not None:
        return "metadata-void-elements"
    if BODY_VOID.search(data) is not None:
        return "body-void-elements"
    return "legacy-void-elements"


def reason_for_axis(axis: str) -> str:
    for axis_info in CASE_AXES:
        if axis_info["axis"] == axis:
            return axis_info["reason"]
    raise SystemExit(f"unknown void-element audit axis: {axis}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
