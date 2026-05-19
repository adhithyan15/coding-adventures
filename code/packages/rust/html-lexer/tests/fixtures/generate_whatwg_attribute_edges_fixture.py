#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer attribute-edge fixture.

The HTML tokenizer attribute states are where small delimiter differences turn
into visible parser-facing tokens and diagnostics. This fixture keeps those
edges pinned across ordinary data-state lexing: quoted and unquoted values,
duplicate names, missing whitespace, NULL replacement, self-closing syntax,
unexpected solidus recovery, and recoverable end-tag attributes.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from generated_fixture_io import write_fixture_json


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-attribute-edges.json")


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    fixture = build_fixture()
    return write_fixture_json(output, fixture, check=args.check)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer attribute-edge fixture JSON."
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
        "format": "whatwg-html-tokenizer-attribute-edges/v1",
        "description": (
            "Attribute tokenizer recovery for quoted and unquoted values, "
            "duplicates, missing whitespace, NULLs, self-closing delimiters, "
            "and recoverable end-tag attributes."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = []
    cases.extend(attribute_value_cases())
    cases.extend(attribute_diagnostic_cases())
    cases.extend(solidus_and_self_closing_cases())
    cases.extend(end_tag_attribute_cases())
    return cases


def attribute_value_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "mixed-quoted-and-unquoted-values",
            "description": "double, single, unquoted, and boolean attributes emit in source order",
            "input": "<a one=\"1\" two='2' three=3 four>",
            "tokens": [
                "StartTag(name=a, attributes=[one=1, two=2, three=3, four=], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "uppercase-tag-and-attribute-names",
            "description": "HTML tag and attribute names are ASCII-lowercased",
            "input": "<DIV CLASS=Hero DATA-ID=42>",
            "tokens": [
                "StartTag(name=div, attributes=[class=Hero, data-id=42], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "empty-quoted-values",
            "description": "empty quoted attributes stay present with empty values",
            "input": "<input disabled value=\"\" data-empty=''>",
            "tokens": [
                "StartTag(name=input, attributes=[disabled=, value=, data-empty=], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "attribute-values-keep-whitespace-inside-quotes",
            "description": "quoted attribute values preserve internal ASCII whitespace",
            "input": "<p title=\"one two\tthree\nfour\">",
            "tokens": [
                "StartTag(name=p, attributes=[title=one two\tthree\nfour], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "character-references-in-attributes",
            "description": "attribute character references decode through the attribute return state",
            "input": "<a title=\"A&amp;B\" data='&#x41;&#65;'>",
            "tokens": [
                "StartTag(name=a, attributes=[title=A&B, data=AA], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "ambiguous-ampersand-in-attribute",
            "description": "semicolonless ambiguous ampersands stay literal in attributes",
            "input": "<a href=\"?x=1&ampy=2\" title=Tom&ampJerry>",
            "tokens": [
                "StartTag(name=a, attributes=[href=?x=1&ampy=2, title=Tom&ampJerry], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "duplicate-attributes-drop-later-name",
            "description": "duplicate attribute names keep the first value and report recovery",
            "input": "<a href=one HREF=two title=ok>",
            "tokens": [
                "StartTag(name=a, attributes=[href=one, title=ok], self_closing=false)",
                "EOF",
            ],
            "diagnostics": ["duplicate-attribute"],
        },
    ]


def attribute_diagnostic_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "unexpected-unquoted-characters",
            "description": "unexpected unquoted value characters are preserved with diagnostics",
            "input": "<a data=one=two sq=x'y lt=x<y tick=x`y dq=x\"y>",
            "tokens": [
                "StartTag(name=a, attributes=[data=one=two, sq=x'y, lt=x<y, tick=x`y, dq=x\"y], self_closing=false)",
                "EOF",
            ],
            "diagnostics": [
                "unexpected-character-in-unquoted-attribute-value",
                "unexpected-character-in-unquoted-attribute-value",
                "unexpected-character-in-unquoted-attribute-value",
                "unexpected-character-in-unquoted-attribute-value",
                "unexpected-character-in-unquoted-attribute-value",
            ],
        },
        {
            "id": "first-unquoted-value-characters",
            "description": "unexpected characters at the beginning of unquoted values are preserved",
            "input": "<a lt=<x eq==x tick=`x ok=value>",
            "tokens": [
                "StartTag(name=a, attributes=[lt=<x, eq==x, tick=`x, ok=value], self_closing=false)",
                "EOF",
            ],
            "diagnostics": [
                "unexpected-character-in-unquoted-attribute-value",
                "unexpected-character-in-unquoted-attribute-value",
                "unexpected-character-in-unquoted-attribute-value",
            ],
        },
        {
            "id": "unexpected-attribute-name-characters",
            "description": "quote and less-than characters in attribute names report but stay literal",
            "input": "<a \"pre=1 mid'dle=2 done <tail=3>",
            "tokens": [
                "StartTag(name=a, attributes=[\"pre=1, mid'dle=2, done=, <tail=3], self_closing=false)",
                "EOF",
            ],
            "diagnostics": [
                "unexpected-character-in-attribute-name",
                "unexpected-character-in-attribute-name",
                "unexpected-character-in-attribute-name",
            ],
        },
        {
            "id": "missing-whitespace-after-quoted-value",
            "description": "quoted attributes without separating whitespace reconsume into new attributes",
            "input": "<a first=\"1\"second=2>",
            "tokens": [
                "StartTag(name=a, attributes=[first=1, second=2], self_closing=false)",
                "EOF",
            ],
            "diagnostics": ["missing-whitespace-between-attributes"],
        },
        {
            "id": "equals-before-attribute-name-after-quoted-value",
            "description": "missing whitespace before equals creates a recoverable equals-named attribute",
            "input": "<a eq=\"x\"==y>",
            "tokens": [
                "StartTag(name=a, attributes=[eq=x, ==y], self_closing=false)",
                "EOF",
            ],
            "diagnostics": [
                "missing-whitespace-between-attributes",
                "unexpected-equals-before-attribute-name",
            ],
        },
        {
            "id": "nulls-in-attribute-values",
            "description": "NULLs in quoted, unquoted, and bare attribute values become replacement characters",
            "input": "<a title=\"x\u0000y\" data=x\u0000y bare=\u0000>",
            "tokens": [
                "StartTag(name=a, attributes=[title=x�y, data=x�y, bare=�], self_closing=false)",
                "EOF",
            ],
            "diagnostics": [
                "unexpected-null-character",
                "unexpected-null-character",
                "unexpected-null-character",
            ],
        },
        {
            "id": "nulls-in-tag-and-attribute-names",
            "description": "NULLs in tag and attribute names become replacement characters",
            "input": "<x\u0000 \u0000=v a\u0000b=1 first \u0000second=2>",
            "tokens": [
                "StartTag(name=x�, attributes=[�=v, a�b=1, first=, �second=2], self_closing=false)",
                "EOF",
            ],
            "diagnostics": [
                "unexpected-null-character",
                "unexpected-null-character",
                "unexpected-null-character",
                "unexpected-null-character",
            ],
        },
    ]


def solidus_and_self_closing_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "self-closing-with-space",
            "description": "a solidus after whitespace sets the self-closing flag",
            "input": "<br />",
            "tokens": ["StartTag(name=br, attributes=[], self_closing=true)", "EOF"],
        },
        {
            "id": "self-closing-without-space",
            "description": "a solidus immediately after the tag name sets the self-closing flag",
            "input": "<br/>",
            "tokens": ["StartTag(name=br, attributes=[], self_closing=true)", "EOF"],
        },
        {
            "id": "double-solidus-before-close",
            "description": "a second solidus before close is reported then preserves self-closing",
            "input": "<br//>",
            "tokens": ["StartTag(name=br, attributes=[], self_closing=true)", "EOF"],
            "diagnostics": ["unexpected-solidus-in-tag"],
        },
        {
            "id": "unexpected-solidus-before-attribute",
            "description": "unexpected solidus before an attribute reconsumes in before-attribute-name",
            "input": "<img/ src=one>",
            "tokens": [
                "StartTag(name=img, attributes=[src=one], self_closing=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-solidus-in-tag"],
        },
        {
            "id": "unexpected-solidus-before-null-attribute",
            "description": "unexpected solidus before a NULL-starting attribute reports both recoveries",
            "input": "<hr/\u0000=x>",
            "tokens": [
                "StartTag(name=hr, attributes=[�=x], self_closing=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-solidus-in-tag", "unexpected-null-character"],
        },
        {
            "id": "solidus-in-unquoted-values",
            "description": "solidus stays value text inside unquoted attribute values",
            "input": "<a href=http://example.test/path data=a/b>",
            "tokens": [
                "StartTag(name=a, attributes=[href=http://example.test/path, data=a/b], self_closing=false)",
                "EOF",
            ],
        },
        {
            "id": "trailing-solidus-inside-unquoted-value",
            "description": "a trailing solidus before tag close belongs to the unquoted value",
            "input": "<img src=cat/>",
            "tokens": [
                "StartTag(name=img, attributes=[src=cat/], self_closing=false)",
                "EOF",
            ],
        },
    ]


def end_tag_attribute_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "end-tag-with-trailing-solidus",
            "description": "end tags with trailing solidus emit the end tag and report recovery",
            "input": "Before</p/>After",
            "tokens": [
                "Text(data=Before)",
                "EndTag(name=p)",
                "Text(data=After)",
                "EOF",
            ],
            "diagnostics": ["end-tag-with-trailing-solidus"],
        },
        {
            "id": "end-tag-with-whitespace-and-solidus",
            "description": "end-tag whitespace before trailing solidus does not add an attribute diagnostic",
            "input": "Before</p />After",
            "tokens": [
                "Text(data=Before)",
                "EndTag(name=p)",
                "Text(data=After)",
                "EOF",
            ],
            "diagnostics": ["end-tag-with-trailing-solidus"],
        },
        {
            "id": "end-tag-with-form-feed-and-solidus",
            "description": "form feed is HTML whitespace before an end-tag trailing solidus",
            "input": "Before</p\u000c/>After",
            "tokens": [
                "Text(data=Before)",
                "EndTag(name=p)",
                "Text(data=After)",
                "EOF",
            ],
            "diagnostics": ["end-tag-with-trailing-solidus"],
        },
        {
            "id": "end-tag-with-attributes",
            "description": "end-tag attributes are ignored after reporting a single diagnostic",
            "input": "Before</p class=x data-y>After",
            "tokens": [
                "Text(data=Before)",
                "EndTag(name=p)",
                "Text(data=After)",
                "EOF",
            ],
            "diagnostics": ["end-tag-with-attributes"],
        },
        {
            "id": "end-tag-with-attributes-and-trailing-solidus",
            "description": "end-tag attributes followed by a solidus stay in attribute recovery",
            "input": "Before</p class=x/>After",
            "tokens": [
                "Text(data=Before)",
                "EndTag(name=p)",
                "Text(data=After)",
                "EOF",
            ],
            "diagnostics": ["end-tag-with-attributes"],
        },
    ]


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
