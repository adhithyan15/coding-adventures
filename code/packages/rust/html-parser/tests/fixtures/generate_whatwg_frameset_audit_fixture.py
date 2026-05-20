#!/usr/bin/env python3

"""Generate Venture's focused WHATWG frameset audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-frameset-audit.json"

FRAMESET_MARKUP = re.compile(r"</?(?:frameset|frame|noframes)(?:\s|/|>)", re.I)

CASE_AXES = (
    {
        "axis": "frameset-shell",
        "reason": "top-level frameset shell creation and trailing boundary recovery",
    },
    {
        "axis": "frame-element",
        "reason": "frame element insertion, void behavior, and frameset nesting",
    },
    {
        "axis": "noframes-content",
        "reason": "noframes RAWTEXT handoff and post-frameset recovery",
    },
    {
        "axis": "body-compatibility",
        "reason": "frameset acceptance or rejection after body-compatible content",
    },
    {
        "axis": "foreign-boundary",
        "reason": "frameset recovery around SVG/MathML foreign-content boundaries",
    },
    {
        "axis": "template-boundary",
        "reason": "frameset and frame handling inside template insertion contexts",
    },
)


@dataclass(frozen=True)
class SmokeCase:
    source: str
    data: str


def main() -> int:
    args = parse_args()
    input_path = Path(args.input).expanduser().resolve()
    output_path = Path(args.output).expanduser().resolve()

    cases = parse_cases(input_path)
    fixture = build_fixture(cases)
    return write_fixture_json(output_path, fixture, check=args.check, ensure_ascii=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate the focused WHATWG frameset audit fixture."
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

        while index < len(lines):
            metadata_line = lines[index]
            index += 1
            if metadata_line == "#document":
                break

        while index < len(lines):
            document_line = lines[index]
            if document_line == "#data" or document_line.startswith("#source "):
                break
            index += 1

        case_data = "\n".join(data)
        if FRAMESET_MARKUP.search(case_data):
            cases.append(SmokeCase(source=source, data=case_data))

    if not cases:
        raise SystemExit(f"{path} does not contain any frameset audit cases")
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
        "format": "whatwg-html-frameset-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction cases "
            "that stress frameset, frame, and noframes recovery."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def axis_for_case(case: SmokeCase) -> str:
    data = case.data.lower()
    if "<template" in data:
        return "template-boundary"
    if "<svg" in data or "<math" in data or "foreignobject" in data:
        return "foreign-boundary"
    if "<noframes" in data or "</noframes" in data:
        return "noframes-content"
    if re.search(r"<frame(?:\s|/|>)", data):
        return "frame-element"
    if re.search(r"<(?:p|div|body|pre|listing|li|dd|dt|button|table|textarea|select|input)\b", data):
        return "body-compatibility"
    return "frameset-shell"


def reason_for_axis(axis: str) -> str:
    for axis_info in CASE_AXES:
        if axis_info["axis"] == axis:
            return axis_info["reason"]
    raise SystemExit(f"unknown frameset audit axis: {axis}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
