#!/usr/bin/env python3

"""Generate Venture's focused WHATWG head/body audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-head-body-audit.json"

HEAD_BODY_MARKUP = re.compile(
    r"</?(?:html|head|body|frameset|base|basefont|bgsound|link|meta|title|style|script)(?:\s|/|>)",
    re.I,
)
HEAD_TEXT_MODE_MARKUP = re.compile(r"</?(?:title|style|script)(?:\s|/|>)", re.I)
HEAD_METADATA_MARKUP = re.compile(
    r"</?(?:base|basefont|bgsound|link|meta)(?:\s|/|>)",
    re.I,
)

CASE_AXES = (
    {
        "axis": "head-text-mode",
        "reason": "title, style, and script handoff around document shell boundaries",
    },
    {
        "axis": "head-metadata-boundary",
        "reason": "metadata elements moving through head, body, and table contexts",
    },
    {
        "axis": "body-frameset-transition",
        "reason": "frameset/body compatibility and frameset-ok transitions",
    },
    {
        "axis": "body-boundary",
        "reason": "explicit body start/end tags and late body handling",
    },
    {
        "axis": "head-boundary",
        "reason": "explicit head start/end tags and misplaced head content",
    },
    {
        "axis": "html-boundary",
        "reason": "html start/end tag merging and stray html boundary recovery",
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
        description="Generate the focused WHATWG head/body audit fixture."
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
        if fragment_context is None and is_head_body_case(case_data):
            cases.append(
                SmokeCase(
                    source=source,
                    data=case_data,
                    fragment_context=fragment_context,
                )
            )

    if not cases:
        raise SystemExit(f"{path} does not contain any head/body audit cases")
    return cases


def is_head_body_case(data: str) -> bool:
    return HEAD_BODY_MARKUP.search(data) is not None


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
        "format": "whatwg-html-head-body-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction cases "
            "that stress html/head/body shell transitions, metadata relocation, "
            "head text-mode handoff, frameset/body compatibility, and late shell tags."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def axis_for_case(case: SmokeCase) -> str:
    data = case.data.lower()

    if HEAD_TEXT_MODE_MARKUP.search(data) is not None:
        return "head-text-mode"
    if HEAD_METADATA_MARKUP.search(data) is not None:
        return "head-metadata-boundary"
    if "<frameset" in data or "</frameset" in data:
        return "body-frameset-transition"
    if "<body" in data or "</body" in data:
        return "body-boundary"
    if "<head" in data or "</head" in data:
        return "head-boundary"
    if "<html" in data or "</html" in data:
        return "html-boundary"
    raise SystemExit(f"unclassified head/body audit case: {case.source}")


def reason_for_axis(axis: str) -> str:
    for axis_info in CASE_AXES:
        if axis_info["axis"] == axis:
            return axis_info["reason"]
    raise SystemExit(f"unknown head/body audit axis: {axis}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
