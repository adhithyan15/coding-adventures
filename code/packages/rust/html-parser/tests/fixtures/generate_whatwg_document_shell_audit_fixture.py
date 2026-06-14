#!/usr/bin/env python3

"""Generate Venture's focused WHATWG document-shell audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-document-shell-audit.json"

DOCTYPE_MARKUP = re.compile(r"<!doctype", re.I)
COMMENT_MARKUP = re.compile(r"<!--|--!?>", re.I)
HTML_MARKUP = re.compile(r"</?html(?:\s|/|>)", re.I)
HEAD_MARKUP = re.compile(r"</?head(?:\s|/|>)", re.I)
BODY_MARKUP = re.compile(r"</?(?:body|frameset|frame)(?:\s|/|>)", re.I)
HEAD_CONTENT_MARKUP = re.compile(
    r"</?(?:base|basefont|bgsound|link|meta|template)(?:\s|/|>)",
    re.I,
)
SHELL_FRAGMENT_CONTEXTS = {"body", "head", "html"}
ANCHOR_SOURCES = {
    "comments01.dat",
    "doctype01.dat",
    "noscript01.dat",
    "tests4.dat",
    "tests6.dat",
    "tests7.dat",
    "tests_innerHTML_1.dat",
}

CASE_AXES = (
    {
        "axis": "doctype-and-quirks",
        "reason": "doctype insertion, force-quirks recovery, and duplicate doctype handling",
    },
    {
        "axis": "html-element-boundary",
        "reason": "explicit html start/end tags and late html-tag recovery",
    },
    {
        "axis": "head-element-boundary",
        "reason": "head insertion, head content, and noscript-in-head recovery",
    },
    {
        "axis": "body-frameset-boundary",
        "reason": "body, frameset, and frame transitions around the document shell",
    },
    {
        "axis": "comment-whitespace-shell",
        "reason": "comments and whitespace around implied html, head, and body nodes",
    },
    {
        "axis": "shell-fragment-context",
        "reason": "html, head, and body fragment parsing contexts",
    },
    {
        "axis": "implicit-document-shell",
        "reason": "implicit html/head/body shell synthesis around legacy inputs",
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
        description="Generate the focused WHATWG document-shell audit fixture."
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
        if is_document_shell_case(source, case_data, fragment_context):
            cases.append(
                SmokeCase(
                    source=source,
                    data=case_data,
                    fragment_context=fragment_context,
                )
            )

    if not cases:
        raise SystemExit(f"{path} does not contain any document-shell audit cases")
    return cases


def is_document_shell_case(
    source: str, data: str, fragment_context: str | None
) -> bool:
    source_file = source.split(":", 1)[0]
    if (
        fragment_context is not None
        and fragment_context.lower() in SHELL_FRAGMENT_CONTEXTS
    ):
        return True
    if source_file == "doctype01.dat":
        return True
    return (
        source_file in ANCHOR_SOURCES
        and (
            DOCTYPE_MARKUP.search(data) is not None
            or COMMENT_MARKUP.search(data) is not None
            or HTML_MARKUP.search(data) is not None
            or HEAD_MARKUP.search(data) is not None
            or BODY_MARKUP.search(data) is not None
            or HEAD_CONTENT_MARKUP.search(data) is not None
        )
    ) or (
        HTML_MARKUP.search(data) is not None
        or HEAD_MARKUP.search(data) is not None
        or BODY_MARKUP.search(data) is not None
    )


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
        "format": "whatwg-html-document-shell-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction cases "
            "that stress doctypes, comments, html/head/body synthesis, frameset "
            "boundaries, and shell fragment contexts."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def axis_for_case(case: SmokeCase) -> str:
    source_file = case.source.split(":", 1)[0]
    fragment_context = (case.fragment_context or "").lower()

    if fragment_context in SHELL_FRAGMENT_CONTEXTS:
        return "shell-fragment-context"
    if source_file == "doctype01.dat" or is_duplicate_doctype_case(case.data):
        return "doctype-and-quirks"
    if HTML_MARKUP.search(case.data):
        return "html-element-boundary"
    if HEAD_MARKUP.search(case.data) or HEAD_CONTENT_MARKUP.search(case.data):
        return "head-element-boundary"
    if BODY_MARKUP.search(case.data):
        return "body-frameset-boundary"
    if COMMENT_MARKUP.search(case.data):
        return "comment-whitespace-shell"
    return "implicit-document-shell"


def is_duplicate_doctype_case(data: str) -> bool:
    lower_data = data.lower()
    return "<!doctype html><!doctype" in lower_data or "<!doctype html></doctype" in lower_data


def reason_for_axis(axis: str) -> str:
    for axis_info in CASE_AXES:
        if axis_info["axis"] == axis:
            return axis_info["reason"]
    raise SystemExit(f"unknown document-shell audit axis: {axis}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
