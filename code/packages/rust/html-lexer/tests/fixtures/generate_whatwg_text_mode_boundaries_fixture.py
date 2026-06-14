#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer text-mode boundary fixture."""

from __future__ import annotations

import argparse
from pathlib import Path

from generated_fixture_io import write_fixture_json


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-text-mode-boundaries.json")


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    return write_fixture_json(output, build_fixture(), check=args.check)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer text-mode boundary fixture JSON."
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
        "format": "whatwg-html-tokenizer-text-mode-boundaries/v1",
        "description": (
            "Parser-seeded RCDATA, RAWTEXT, and PLAINTEXT text-mode boundaries "
            "for less-than, end-tag-open, end-tag-name, NULL, EOF, and "
            "literal markup recovery."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    return [
        *rcdata_cases(),
        *rawtext_cases(),
        *plaintext_cases(),
    ]


def rcdata_cases() -> list[dict[str, object]]:
    return [
        state(
            "rcdata-null-reference-and-delimiter",
            "RCDATA state",
            "a\u0000&amp;</title>tail",
            ["Text(data=a�&)", "EndTag(name=title)", "Text(data=tail)", "EOF"],
            last_start_tag="title",
            diagnostics=["unexpected-null-character"],
        ),
        state(
            "rcdata-ambiguous-ampersand-before-delimiter",
            "RCDATA state",
            "a&notanentity;</title>",
            ["Text(data=a¬anentity;)", "EndTag(name=title)", "EOF"],
            last_start_tag="title",
            diagnostics=["missing-semicolon-after-character-reference"],
        ),
        state(
            "rcdata-less-than-end-tag",
            "RCDATA less-than sign state",
            "/title>tail",
            ["EndTag(name=title)", "Text(data=tail)", "EOF"],
            last_start_tag="title",
        ),
        state(
            "rcdata-less-than-non-slash",
            "RCDATA less-than sign state",
            "x</title>",
            ["Text(data=<x)", "EndTag(name=title)", "EOF"],
            last_start_tag="title",
        ),
        state(
            "rcdata-less-than-eof",
            "RCDATA less-than sign state",
            "",
            ["Text(data=<)", "EOF"],
            last_start_tag="title",
        ),
        state(
            "rcdata-end-tag-open-matching",
            "RCDATA end tag open state",
            "title>tail",
            ["EndTag(name=title)", "Text(data=tail)", "EOF"],
            last_start_tag="title",
        ),
        state(
            "rcdata-end-tag-open-mismatch",
            "RCDATA end tag open state",
            "style>tail</title>",
            ["Text(data=</style>tail)", "EndTag(name=title)", "EOF"],
            last_start_tag="title",
        ),
        state(
            "rcdata-end-tag-open-eof",
            "RCDATA end tag open state",
            "",
            ["Text(data=</)", "EOF"],
            last_start_tag="title",
        ),
        state(
            "rcdata-end-tag-name-delimiter",
            "RCDATA end tag name state",
            ">tail",
            ["EndTag(name=title)", "Text(data=tail)", "EOF"],
            last_start_tag="title",
            current_end_tag="title",
            temporary_buffer="title",
        ),
        state(
            "rcdata-end-tag-name-attribute-recovery",
            "RCDATA end tag name state",
            " class=x>tail",
            ["EndTag(name=title)", "Text(data=tail)", "EOF"],
            last_start_tag="title",
            current_end_tag="title",
            temporary_buffer="title",
            diagnostics=["end-tag-with-attributes"],
        ),
        state(
            "rcdata-end-tag-name-mismatch-eof",
            "RCDATA end tag name state",
            "",
            ["Text(data=</style)", "EOF"],
            last_start_tag="title",
            current_end_tag="style",
            temporary_buffer="style",
        ),
    ]


def rawtext_cases() -> list[dict[str, object]]:
    return [
        state(
            "rawtext-ampersand-stays-literal-before-delimiter",
            "RAWTEXT state",
            "a&amp;</style>tail",
            ["Text(data=a&amp;)", "EndTag(name=style)", "Text(data=tail)", "EOF"],
            last_start_tag="style",
        ),
        state(
            "rawtext-null-before-delimiter",
            "RAWTEXT state",
            "a\u0000b</style>",
            ["Text(data=a�b)", "EndTag(name=style)", "EOF"],
            last_start_tag="style",
            diagnostics=["unexpected-null-character"],
        ),
        state(
            "rawtext-less-than-end-tag",
            "RAWTEXT less-than sign state",
            "/style>tail",
            ["EndTag(name=style)", "Text(data=tail)", "EOF"],
            last_start_tag="style",
        ),
        state(
            "rawtext-less-than-non-slash",
            "RAWTEXT less-than sign state",
            "x</style>",
            ["Text(data=<x)", "EndTag(name=style)", "EOF"],
            last_start_tag="style",
        ),
        state(
            "rawtext-less-than-eof",
            "RAWTEXT less-than sign state",
            "",
            ["Text(data=<)", "EOF"],
            last_start_tag="style",
        ),
        state(
            "rawtext-end-tag-open-matching",
            "RAWTEXT end tag open state",
            "style>tail",
            ["EndTag(name=style)", "Text(data=tail)", "EOF"],
            last_start_tag="style",
        ),
        state(
            "rawtext-end-tag-open-mismatch",
            "RAWTEXT end tag open state",
            "title>tail</style>",
            ["Text(data=</title>tail)", "EndTag(name=style)", "EOF"],
            last_start_tag="style",
        ),
        state(
            "rawtext-end-tag-open-eof",
            "RAWTEXT end tag open state",
            "",
            ["Text(data=</)", "EOF"],
            last_start_tag="style",
        ),
        state(
            "rawtext-end-tag-name-delimiter",
            "RAWTEXT end tag name state",
            ">tail",
            ["EndTag(name=style)", "Text(data=tail)", "EOF"],
            last_start_tag="style",
            current_end_tag="style",
            temporary_buffer="style",
        ),
        state(
            "rawtext-end-tag-name-self-closing-recovery",
            "RAWTEXT end tag name state",
            "/>tail",
            ["EndTag(name=style)", "Text(data=tail)", "EOF"],
            last_start_tag="style",
            current_end_tag="style",
            temporary_buffer="style",
            diagnostics=["end-tag-with-trailing-solidus"],
        ),
        state(
            "rawtext-end-tag-name-mismatch-eof",
            "RAWTEXT end tag name state",
            "",
            ["Text(data=</title)", "EOF"],
            last_start_tag="style",
            current_end_tag="title",
            temporary_buffer="title",
        ),
    ]


def plaintext_cases() -> list[dict[str, object]]:
    return [
        state(
            "plaintext-markup-stays-text",
            "PLAINTEXT state",
            "<b>x</b></plaintext>",
            ["Text(data=<b>x</b></plaintext>)", "EOF"],
        ),
        state(
            "plaintext-ampersand-stays-literal",
            "PLAINTEXT state",
            "Tom &amp; Jerry",
            ["Text(data=Tom &amp; Jerry)", "EOF"],
        ),
        state(
            "plaintext-null-replacement",
            "PLAINTEXT state",
            "a\u0000b",
            ["Text(data=a�b)", "EOF"],
            diagnostics=["unexpected-null-character"],
        ),
        state(
            "plaintext-less-than-and-slash-at-eof",
            "PLAINTEXT state",
            "alpha</",
            ["Text(data=alpha</)", "EOF"],
        ),
        state(
            "plaintext-comment-looking-text",
            "PLAINTEXT state",
            "<!--x--><p>y",
            ["Text(data=<!--x--><p>y)", "EOF"],
        ),
        state(
            "plaintext-cdata-looking-text",
            "PLAINTEXT state",
            "<![CDATA[x]]>",
            ["Text(data=<![CDATA[x]]>)", "EOF"],
        ),
        state(
            "plaintext-preserves-crlf-normalized-input",
            "PLAINTEXT state",
            "a\r\nb",
            ["Text(data=a\nb)", "EOF"],
        ),
    ]


def state(
    case_id: str,
    initial_state: str,
    input_text: str,
    tokens: list[str],
    *,
    diagnostics: list[str] | None = None,
    last_start_tag: str | None = None,
    current_end_tag: str | None = None,
    temporary_buffer: str | None = None,
) -> dict[str, object]:
    item: dict[str, object] = {
        "id": case_id,
        "description": f"{initial_state} boundary case `{case_id}`",
        "input": input_text,
        "initial_state": initial_state,
        "tokens": tokens,
    }
    if diagnostics is not None:
        item["diagnostics"] = diagnostics
    if last_start_tag is not None:
        item["last_start_tag"] = last_start_tag
    if current_end_tag is not None:
        item["current_end_tag"] = current_end_tag
    if temporary_buffer is not None:
        item["temporary_buffer"] = temporary_buffer
    return item


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
