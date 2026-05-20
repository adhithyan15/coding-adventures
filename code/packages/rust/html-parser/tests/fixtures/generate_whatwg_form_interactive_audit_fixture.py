#!/usr/bin/env python3

"""Generate Venture's focused WHATWG form/interactive audit fixture."""

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
DEFAULT_OUTPUT = FIXTURE_DIR / "whatwg-form-interactive-audit.json"

FORM_INTERACTIVE_MARKUP = re.compile(
    r"</?(?:a|button|fieldset|form|input|label|nobr|optgroup|option|select|textarea)(?:\s|/|>)",
    re.I,
)
FORM_INTERACTIVE_FRAGMENT_CONTEXTS = {
    "a",
    "button",
    "form",
    "nobr",
    "optgroup",
    "option",
    "select",
    "textarea",
}

CASE_AXES = (
    {
        "axis": "interactive-formatting",
        "reason": "anchor and nobr adoption-agency or repeated-interactive recovery",
    },
    {
        "axis": "button-boundary",
        "reason": "button scoping, repeated button starts, and button boundary recovery",
    },
    {
        "axis": "form-control",
        "reason": "form, input, label, and fieldset insertion recovery",
    },
    {
        "axis": "select-option",
        "reason": "select, option, and optgroup insertion and implied-end recovery",
    },
    {
        "axis": "textarea-rawtext",
        "reason": "textarea RCDATA handoff and form-associated text control recovery",
    },
    {
        "axis": "fragment-context",
        "reason": "fragment parsing in form and interactive element contexts",
    },
    {
        "axis": "stray-interactive-end-tags",
        "reason": "stray form/interactive end-tag recovery around ordinary and table content",
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
        description="Generate the focused WHATWG form/interactive audit fixture."
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
        if is_form_interactive_case(case_data, fragment_context):
            cases.append(
                SmokeCase(
                    source=source,
                    data=case_data,
                    fragment_context=fragment_context,
                )
            )

    if not cases:
        raise SystemExit(f"{path} does not contain any form/interactive audit cases")
    return cases


def is_form_interactive_case(data: str, fragment_context: str | None) -> bool:
    if (
        fragment_context is not None
        and fragment_context.lower() in FORM_INTERACTIVE_FRAGMENT_CONTEXTS
    ):
        return True
    return FORM_INTERACTIVE_MARKUP.search(data) is not None


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
        "format": "whatwg-html-form-interactive-audit/v1",
        "description": (
            "Focused parser audit over selected html5lib tree-construction cases "
            "that stress form-associated controls and interactive-element recovery."
        ),
        "source_fixture": "html5lib-tree-construction-smoke.dat",
        "case_count": len(selected),
        "counts_by_axis": counts_by_axis,
        "cases": selected,
    }


def axis_for_case(case: SmokeCase) -> str:
    data = case.data.lower()
    fragment_context = (case.fragment_context or "").lower()

    if fragment_context in FORM_INTERACTIVE_FRAGMENT_CONTEXTS:
        return "fragment-context"
    if "<textarea" in data:
        return "textarea-rawtext"
    if re.search(r"<(?:select|option|optgroup)(?:\s|/|>)", data):
        return "select-option"
    if re.search(r"<button(?:\s|/|>)", data):
        return "button-boundary"
    if re.search(r"<(?:form|input|label|fieldset)(?:\s|/|>)", data):
        return "form-control"
    if re.search(r"<(?:a|nobr)(?:\s|/|>)", data):
        return "interactive-formatting"
    return "stray-interactive-end-tags"


def reason_for_axis(axis: str) -> str:
    for axis_info in CASE_AXES:
        if axis_info["axis"] == axis:
            return axis_info["reason"]
    raise SystemExit(f"unknown form/interactive audit axis: {axis}")


def stable_case_id(source: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", source.lower()).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
