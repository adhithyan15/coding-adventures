#!/usr/bin/env python3

"""Generate Venture's focused WHATWG foreign-content audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-foreign-audit.json"

CORE_FOREIGN_MARKUP = re.compile(
    r"</?(?:svg|math|foreignObject|annotation-xml|mi|mo|mn|ms|mtext|malignmark|mglyph|circle|path)(?:\s|/|>)",
    re.I,
)
SVG_MARKUP = re.compile(
    r"</?(?:svg|foreignObject|desc|title|font|image|circle|path)(?:\s|/|>)",
    re.I,
)
MATHML_MARKUP = re.compile(
    r"</?(?:math|mi|mo|mn|ms|mtext|malignmark|mglyph|annotation-xml)(?:\s|/|>)",
    re.I,
)
HTML_INTEGRATION_MARKUP = re.compile(
    r"</?(?:foreignObject|desc|title|annotation-xml)(?:\s|/|>)",
    re.I,
)
TABLE_MARKUP = re.compile(
    r"</?(?:table|tbody|thead|tfoot|tr|td|th|caption|colgroup|col)(?:\s|/|>)",
    re.I,
)
FOREIGN_FRAGMENT_CONTEXTS = {
    "annotation-xml",
    "foreignobject",
    "math",
    "svg",
}

CASE_AXES = (
    {
        "axis": "svg-boundary",
        "reason": "SVG insertion-mode boundaries, special tags, and CDATA recovery",
    },
    {
        "axis": "mathml-boundary",
        "reason": "MathML insertion-mode boundaries and text-integration recovery",
    },
    {
        "axis": "html-integration-point",
        "reason": "foreignObject, desc/title, and annotation-xml HTML integration points",
    },
    {
        "axis": "table-foreign-boundary",
        "reason": "foreign-content tokens crossing table insertion-mode boundaries",
    },
    {
        "axis": "foreign-fragment",
        "reason": "SVG and MathML fragment parsing with foreign-content context seeding",
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
        description="Generate the focused WHATWG foreign-content audit fixture."
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
        if is_foreign_content_case(source, case_data, fragment_context):
            cases.append(
                SmokeCase(
                    source=source,
                    data=case_data,
                    fragment_context=fragment_context,
                )
            )

    if not cases:
        raise SystemExit(f"{path} does not contain any foreign-content audit cases")
    return cases


def is_foreign_content_case(
    source: str, data: str, fragment_context: str | None
) -> bool:
    if source.startswith("foreign-fragment.dat:"):
        return True
    if is_foreign_fragment_context(fragment_context):
        return True
    return CORE_FOREIGN_MARKUP.search(data) is not None


def is_foreign_fragment_context(fragment_context: str | None) -> bool:
    if fragment_context is None:
        return False
    normalized = fragment_context.lower()
    namespace = normalized.split(maxsplit=1)[0]
    return normalized in FOREIGN_FRAGMENT_CONTEXTS or namespace in {"math", "svg"}


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
        "format": "whatwg-html-foreign-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction cases "
            "that stress SVG, MathML, foreign fragments, HTML integration points, "
            "and table/foreign-content boundaries."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def axis_for_case(case: SmokeCase) -> str:
    fragment_context = (case.fragment_context or "").lower()

    if case.source.startswith("foreign-fragment.dat:"):
        return "foreign-fragment"
    if is_foreign_fragment_context(case.fragment_context):
        return "foreign-fragment"
    if HTML_INTEGRATION_MARKUP.search(case.data):
        return "html-integration-point"
    if TABLE_MARKUP.search(case.data):
        return "table-foreign-boundary"
    if MATHML_MARKUP.search(case.data):
        return "mathml-boundary"
    if SVG_MARKUP.search(case.data):
        return "svg-boundary"
    raise SystemExit(f"unclassified foreign-content audit case: {case.source}")


def reason_for_axis(axis: str) -> str:
    for axis_info in CASE_AXES:
        if axis_info["axis"] == axis:
            return axis_info["reason"]
    raise SystemExit(f"unknown foreign-content audit axis: {axis}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
