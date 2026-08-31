#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer tag-open recovery fixture.

Tag-open and tag-name states decide whether `<` starts markup, remains text, or
falls into bogus-comment recovery. This fixture pins the visible lexer contract
for ordinary start/end tags, ASCII casing, tag whitespace, invalid openers,
NULL replacement in tag names, and EOF recovery for partial tag tokens.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from generated_fixture_io import write_fixture_json


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-tag-open-recovery.json")


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    fixture = build_fixture()
    return write_fixture_json(output, fixture, check=args.check)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer tag-open recovery fixture JSON."
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
        "format": "whatwg-html-tokenizer-tag-open-recovery/v1",
        "description": (
            "Tag-open and tag-name recovery for ordinary tags, ASCII casing, "
            "HTML whitespace delimiters, invalid openers, NULL replacement, "
            "and EOF partial-token drops."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = []
    cases.extend(ordinary_tag_cases())
    cases.extend(tag_name_recovery_cases())
    cases.extend(invalid_tag_open_cases())
    cases.extend(eof_recovery_cases())
    return cases


def ordinary_tag_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "basic-start-text-end",
            "description": "ordinary start and end tags bracket text",
            "input": "<p>Hello</p>",
            "tokens": [
                "StartTag(name=p, attributes=[], self_closing=false)",
                "Text(data=Hello)",
                "EndTag(name=p)",
                "EOF",
            ],
        },
        {
            "id": "uppercase-start-and-end-tags",
            "description": "tag names are ASCII-lowercased in start and end tags",
            "input": "<DIV>Hi</DIV>",
            "tokens": [
                "StartTag(name=div, attributes=[], self_closing=false)",
                "Text(data=Hi)",
                "EndTag(name=div)",
                "EOF",
            ],
        },
        {
            "id": "mixed-case-tag-name",
            "description": "mixed-case tag names preserve non-ASCII text while lowering ASCII",
            "input": "<My-Tag>ok</My-Tag>",
            "tokens": [
                "StartTag(name=my-tag, attributes=[], self_closing=false)",
                "Text(data=ok)",
                "EndTag(name=my-tag)",
                "EOF",
            ],
        },
        {
            "id": "start-tag-tab-whitespace",
            "description": "tab after a tag name enters attribute parsing",
            "input": "<p\tclass=x>",
            "tokens": [
                "StartTag(name=p, attributes=[class=x], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "start-tag-newline-whitespace",
            "description": "line feed after a tag name enters attribute parsing",
            "input": "<p\nclass=x>",
            "tokens": [
                "StartTag(name=p, attributes=[class=x], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "start-tag-form-feed-whitespace",
            "description": "form feed is HTML whitespace after a tag name",
            "input": "<p\u000cclass=x\u000c/>",
            "tokens": [
                "StartTag(name=p, attributes=[class=x], self_closing=true)",
                "EOF",
            ],
        },
        {
            "id": "end-tag-form-feed-before-close",
            "description": "form feed is HTML whitespace after an end-tag name",
            "input": "</p\u000c>",
            "tokens": ["EndTag(name=p)", "EOF"],
        },
        {
            "id": "self-closing-start-tag",
            "description": "solidus after tag name marks a self-closing start tag",
            "input": "<br/>",
            "tokens": ["StartTag(name=br, attributes=[], self_closing=true)", "EOF"],
        },
    ]


def tag_name_recovery_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "null-in-start-tag-name",
            "description": "NULL inside a start-tag name becomes U+FFFD",
            "input": "<x\u0000>",
            "tokens": ["StartTag(name=x�, attributes=[], self_closing=false)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "null-in-end-tag-name",
            "description": "NULL inside an end-tag name becomes U+FFFD",
            "input": "</x\u0000>",
            "tokens": ["EndTag(name=x�)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "nulls-in-matching-tag-pair",
            "description": "matching start and end tag names both replace NULLs",
            "input": "<x\u0000>body</x\u0000>",
            "tokens": [
                "StartTag(name=x�, attributes=[], self_closing=false)",
                "Text(data=body)",
                "EndTag(name=x�)",
                "EOF",
            ],
            "diagnostics": ["unexpected-null-character", "unexpected-null-character"],
        },
    ]


def invalid_tag_open_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "less-than-space-stays-text",
            "description": "less-than followed by space is text plus a diagnostic",
            "input": "Before < after",
            "tokens": ["Text(data=Before )", "Text(data=< after)", "EOF"],
            "diagnostics": ["invalid-first-character-of-tag-name"],
        },
        {
            "id": "less-than-equals-stays-text",
            "description": "less-than followed by equals is not a tag opener",
            "input": "a<=b",
            "tokens": ["Text(data=a)", "Text(data=<=b)", "EOF"],
            "diagnostics": ["invalid-first-character-of-tag-name"],
        },
        {
            "id": "less-than-digit-stays-text",
            "description": "less-than followed by a digit is text recovery",
            "input": "a<3b",
            "tokens": ["Text(data=a)", "Text(data=<3b)", "EOF"],
            "diagnostics": ["invalid-first-character-of-tag-name"],
        },
        {
            "id": "less-than-null-reconsumes-as-text",
            "description": "NULL after less-than reports invalid tag open then data NULL recovery",
            "input": "Before <\u0000 after",
            "tokens": ["Text(data=Before )", "Text(data=<\u0000 after)", "EOF"],
            "diagnostics": [
                "invalid-first-character-of-tag-name",
                "unexpected-null-character",
            ],
        },
        {
            "id": "invalid-end-tag-digit-bogus-comment",
            "description": "end-tag open followed by a digit recovers as a bogus comment",
            "input": "Before</3>After",
            "tokens": [
                "Text(data=Before)",
                "Comment(data=3)",
                "Text(data=After)",
                "EOF",
            ],
            "diagnostics": ["invalid-first-character-of-tag-name"],
        },
        {
            "id": "invalid-end-tag-space-bogus-comment",
            "description": "end-tag open followed by whitespace recovers as a bogus comment",
            "input": "Before</ nope>After",
            "tokens": [
                "Text(data=Before)",
                "Comment(data= nope)",
                "Text(data=After)",
                "EOF",
            ],
            "diagnostics": ["invalid-first-character-of-tag-name"],
        },
    ]


def eof_recovery_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "eof-after-less-than",
            "description": "EOF after a lone less-than preserves text recovery",
            "input": "Before<",
            "tokens": ["Text(data=Before)", "Text(data=<)", "EOF"],
            "diagnostics": ["eof-in-tag-open-state"],
        },
        {
            "id": "eof-after-end-tag-open",
            "description": "EOF after an end-tag opener preserves literal text recovery",
            "input": "Before</",
            "tokens": ["Text(data=Before)", "Text(data=</)", "EOF"],
            "diagnostics": ["eof-in-end-tag-open-state"],
        },
        {
            "id": "eof-in-start-tag-name",
            "description": "EOF in a start-tag name drops the partial start tag",
            "input": "<div",
            "tokens": ["EOF"],
            "diagnostics": ["eof-in-tag-name-state"],
        },
        {
            "id": "eof-in-start-tag-attribute",
            "description": "EOF in start-tag attributes drops the partial start tag",
            "input": "<div class=\"open",
            "tokens": ["EOF"],
            "diagnostics": ["eof-in-tag"],
        },
        {
            "id": "eof-in-end-tag-name",
            "description": "EOF in an end-tag name drops the partial end tag",
            "input": "</section",
            "tokens": ["EOF"],
            "diagnostics": ["eof-in-end-tag-name-state"],
        },
        {
            "id": "eof-in-end-tag-attributes",
            "description": "EOF in end-tag attributes drops the partial end tag",
            "input": "</section class=x",
            "tokens": ["EOF"],
            "diagnostics": ["end-tag-with-attributes", "eof-in-end-tag-name-state"],
        },
    ]


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
