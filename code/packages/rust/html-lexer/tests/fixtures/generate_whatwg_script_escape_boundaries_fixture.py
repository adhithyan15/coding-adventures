#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer script escape boundary fixture."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-script-escape-boundaries.json")


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    text = json.dumps(build_fixture(), indent=2, ensure_ascii=False, sort_keys=True) + "\n"

    if args.check:
        if output.read_text() != text:
            raise SystemExit(f"{output} is stale; regenerate it")
        return 0

    output.write_text(text)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer script escape boundary fixture JSON."
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


def build_fixture() -> dict[str, object]:
    return {
        "format": "whatwg-html-tokenizer-script-escape-boundaries/v1",
        "description": (
            "Script data escaped, double-escaped, escape-start, escape-end, "
            "NULL, EOF, and seeded continuation boundary recovery."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    return [
        data("script-data-comment-open-then-end-tag", "a<!--b</script>c", ["Text(data=a<!--b)", "EndTag(name=script)", "Text(data=c)", "EOF"]),
        data("script-data-comment-close-returns-to-script", "a<!--b-->c</script>", ["Text(data=a<!--b-->c)", "EndTag(name=script)", "EOF"]),
        data("script-data-comment-eof", "a<!--open", ["Text(data=a<!--open)", "EOF"], diagnostics=["eof-in-script-html-comment-like-text"]),
        data("script-data-comment-null", "a<!--x\u0000y</script>", ["Text(data=a<!--x�y)", "EndTag(name=script)", "EOF"], diagnostics=["unexpected-null-character"]),
        data("script-data-double-escaped-inner-script", "a<!--<script>x</script>y</script>", ["Text(data=a<!--<script>x</script>y)", "EndTag(name=script)", "EOF"]),
        state("script-less-than-non-end", "Script data less-than sign state", "x</script>", ["Text(data=<x)", "EndTag(name=script)", "EOF"]),
        state("script-less-than-eof", "Script data less-than sign state", "", ["Text(data=<)", "EOF"]),
        state("script-less-than-end-tag-open", "Script data less-than sign state", "/script>tail", ["EndTag(name=script)", "Text(data=tail)", "EOF"]),
        state("script-escape-start-nondash", "Script data escape start state", "x</script>", ["Text(data=x)", "EndTag(name=script)", "EOF"]),
        state("script-escape-start-eof", "Script data escape start state", "", ["EOF"]),
        state("script-escape-start-dash-dash", "Script data escape start state", "--x</script>", ["Text(data=--x)", "EndTag(name=script)", "EOF"]),
        state("script-escape-start-dash-nondash", "Script data escape start dash state", "x</script>", ["Text(data=x)", "EndTag(name=script)", "EOF"]),
        state("script-escape-start-dash-eof", "Script data escape start dash state", "", ["EOF"]),
        state("script-escaped-end-tag", "Script data escaped state", "x</script>tail", ["Text(data=x)", "EndTag(name=script)", "Text(data=tail)", "EOF"]),
        state("script-escaped-mismatched-end-tag", "Script data escaped state", "x</style>y</script>", ["Text(data=x</style>y)", "EndTag(name=script)", "EOF"]),
        state("script-escaped-comment-close", "Script data escaped state", "x-->y</script>", ["Text(data=x-->y)", "EndTag(name=script)", "EOF"]),
        state("script-escaped-null", "Script data escaped state", "x\u0000y</script>", ["Text(data=x�y)", "EndTag(name=script)", "EOF"], diagnostics=["unexpected-null-character"]),
        state("script-escaped-eof", "Script data escaped state", "x", ["Text(data=x)", "EOF"], diagnostics=["eof-in-script-html-comment-like-text"]),
        state("script-escaped-dash-second-dash", "Script data escaped dash state", "-x</script>", ["Text(data=-x)", "EndTag(name=script)", "EOF"]),
        state("script-escaped-dash-less-than", "Script data escaped dash state", "<x</script>", ["Text(data=<x)", "EndTag(name=script)", "EOF"]),
        state("script-escaped-dash-null", "Script data escaped dash state", "\u0000x</script>", ["Text(data=�x)", "EndTag(name=script)", "EOF"], diagnostics=["unexpected-null-character"]),
        state("script-escaped-dash-eof", "Script data escaped dash state", "", ["EOF"], diagnostics=["eof-in-script-html-comment-like-text"]),
        state("script-escaped-dash-dash-close", "Script data escaped dash dash state", ">x</script>", ["Text(data=>x)", "EndTag(name=script)", "EOF"]),
        state("script-escaped-dash-dash-more-dashes", "Script data escaped dash dash state", "-->x</script>", ["Text(data=-->x)", "EndTag(name=script)", "EOF"]),
        state("script-escaped-dash-dash-null", "Script data escaped dash dash state", "\u0000x</script>", ["Text(data=�x)", "EndTag(name=script)", "EOF"], diagnostics=["unexpected-null-character"]),
        state("script-escaped-less-than-non-special", "Script data escaped less-than sign state", "x</script>", ["Text(data=<x)", "EndTag(name=script)", "EOF"]),
        state("script-escaped-less-than-slash-end", "Script data escaped less-than sign state", "/script>tail", ["EndTag(name=script)", "Text(data=tail)", "EOF"]),
        state("script-escaped-less-than-uppercase-script", "Script data escaped less-than sign state", "SCRIPT>x</script>tail</script>", ["Text(data=<SCRIPT>x</script>tail)", "EndTag(name=script)", "EOF"]),
        state("script-escaped-less-than-nonscript", "Script data escaped less-than sign state", "style>x</script>", ["Text(data=<style>x)", "EndTag(name=script)", "EOF"]),
        state("script-double-escape-start-script", "Script data double escape start state", ">x</script>tail</script>", ["Text(data=>x</script>tail)", "EndTag(name=script)", "EOF"], temporary_buffer="script"),
        state("script-double-escape-start-nonscript", "Script data double escape start state", ">x</script>", ["Text(data=>x)", "EndTag(name=script)", "EOF"], temporary_buffer="style"),
        state("script-double-escape-start-eof", "Script data double escape start state", "", ["EOF"], temporary_buffer="script"),
        state("script-double-escaped-end-script", "Script data double escaped state", "x</script>tail</script>", ["Text(data=x</script>tail)", "EndTag(name=script)", "EOF"]),
        state("script-double-escaped-null", "Script data double escaped state", "x\u0000y</script>tail</script>", ["Text(data=x�y</script>tail)", "EndTag(name=script)", "EOF"], diagnostics=["unexpected-null-character"]),
        state("script-double-escaped-eof", "Script data double escaped state", "x", ["Text(data=x)", "EOF"], diagnostics=["eof-in-script-html-comment-like-text"]),
        state("script-double-escaped-dash-dash-close", "Script data double escaped dash dash state", ">x</script>", ["Text(data=>x)", "EndTag(name=script)", "EOF"]),
        state("script-double-escaped-dash-null", "Script data double escaped dash state", "\u0000x</script>tail</script>", ["Text(data=�x</script>tail)", "EndTag(name=script)", "EOF"], diagnostics=["unexpected-null-character"]),
        state("script-double-escaped-less-than-slash-script", "Script data double escaped less-than sign state", "/script>tail</script>", ["Text(data=/script>tail)", "EndTag(name=script)", "EOF"]),
        state("script-double-escaped-less-than-nonslash", "Script data double escaped less-than sign state", "x</script>tail</script>", ["Text(data=x</script>tail)", "EndTag(name=script)", "EOF"]),
        state("script-double-escape-end-script", "Script data double escape end state", ">tail</script>", ["Text(data=>tail)", "EndTag(name=script)", "EOF"], temporary_buffer="script"),
        state("script-double-escape-end-nonscript", "Script data double escape end state", ">tail</script>outer</script>", ["Text(data=>tail</script>outer)", "EndTag(name=script)", "EOF"], temporary_buffer="style"),
        state("script-double-escape-end-eof", "Script data double escape end state", "", ["EOF"], temporary_buffer="script"),
    ]


def data(
    case_id: str,
    input_text: str,
    tokens: list[str],
    *,
    diagnostics: list[str] | None = None,
) -> dict[str, object]:
    return state(
        case_id,
        "Script data state",
        input_text,
        tokens,
        diagnostics=diagnostics,
    )


def state(
    case_id: str,
    initial_state: str,
    input_text: str,
    tokens: list[str],
    *,
    diagnostics: list[str] | None = None,
    temporary_buffer: str | None = None,
) -> dict[str, object]:
    item: dict[str, object] = {
        "id": case_id,
        "description": f"{initial_state} boundary case `{case_id}`",
        "input": input_text,
        "initial_state": initial_state,
        "last_start_tag": "script",
        "tokens": tokens,
    }
    if diagnostics is not None:
        item["diagnostics"] = diagnostics
    if temporary_buffer is not None:
        item["temporary_buffer"] = temporary_buffer
    return item


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
