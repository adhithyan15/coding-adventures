#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer chunk-boundary fixture.

HTML tokenization is defined over an input stream. The runtime accepts chunks,
so the observable token stream must not depend on where the embedding splits
that stream. This generated fixture keeps that streaming invariant explicit
across representative tokenizer states and recovery paths.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-chunk-boundaries.json")

CASES = [
    {
        "id": "data-tags-and-text",
        "description": "data state text and ordinary tags",
        "input": "alpha<p class=x>β</p>omega",
    },
    {
        "id": "unicode-text",
        "description": "multi-byte Unicode scalar boundaries",
        "input": "AéΩ🦀Z",
    },
    {
        "id": "self-closing-and-void-like",
        "description": "self-closing syntax and following text",
        "input": "<br/><img src=one/>tail",
    },
    {
        "id": "tag-name-reconsume",
        "description": "tag-open reconsume paths",
        "input": "a</p><not-a-known-tag data-x=y>",
    },
    {
        "id": "attribute-quote-mix",
        "description": "quoted and unquoted attribute values",
        "input": "<a href='one&copy;' title=\"two &#65;\" data-x=three/four>",
    },
    {
        "id": "ambiguous-attribute-ampersand",
        "description": "ambiguous ampersand stays literal in attributes",
        "input": "<a title='&copycat &notin &ampersand'>",
    },
    {
        "id": "text-named-references",
        "description": "named character references in data",
        "input": "a&amp;b &copycat &notin; &CounterClockwiseContourIntegral;",
    },
    {
        "id": "text-numeric-references",
        "description": "numeric character references in data",
        "input": "A=&#65; grin=&#x1F600; null=&#0; no-digits=&#x;",
    },
    {
        "id": "comments",
        "description": "comment start, body, and close states",
        "input": "<!--alpha<!--beta--!>tail",
    },
    {
        "id": "bogus-comment-question-mark",
        "description": "bogus comment from question-mark tag opener",
        "input": "<?alpha beta>",
    },
    {
        "id": "markup-declaration-cdata",
        "description": "CDATA markup declaration in HTML lexer",
        "input": "<![CDATA[alpha < beta]]>tail",
    },
    {
        "id": "doctype-public-system",
        "description": "DOCTYPE public and system identifiers",
        "input": '<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN" "about:legacy-compat">',
    },
    {
        "id": "doctype-recovery",
        "description": "DOCTYPE diagnostics and force-quirks recovery",
        "input": '<!DOCTYPE html PUBLIC alpha "system">',
    },
    {
        "id": "unexpected-solidus-before-attribute",
        "description": "unexpected solidus recovery before later attributes",
        "input": "<img/ src=x alt=y>",
    },
    {
        "id": "duplicate-attributes",
        "description": "duplicate attribute recovery",
        "input": "<a href=one HREF=two title=ok>",
    },
    {
        "id": "unexpected-null-replacement",
        "description": "NULL replacement in data and attribute values",
        "input": "a\u0000b<p title='c\u0000d'>",
    },
    {
        "id": "rcdata-title",
        "description": "RCDATA entity decoding and matching end tag",
        "input": "alpha &copy; </title>",
        "initial_state": "RCDATA state",
        "last_start_tag": "title",
    },
    {
        "id": "rcdata-mismatched-end-tag",
        "description": "RCDATA mismatched end tag stays text",
        "input": "alpha </titlX attr=x> omega</title>",
        "initial_state": "RCDATA state",
        "last_start_tag": "title",
    },
    {
        "id": "rawtext-style",
        "description": "RAWTEXT content and matching end tag",
        "input": "a < b && c > d</style>",
        "initial_state": "RAWTEXT state",
        "last_start_tag": "style",
    },
    {
        "id": "rawtext-mismatched-end-tag",
        "description": "RAWTEXT mismatched end tag stays text",
        "input": "a</styLX>z</style>",
        "initial_state": "RAWTEXT state",
        "last_start_tag": "style",
    },
    {
        "id": "script-data-basic",
        "description": "script data text and matching end tag",
        "input": "if (a < b) { c(); }</script>",
        "initial_state": "Script data state",
        "last_start_tag": "script",
    },
    {
        "id": "script-data-escaped",
        "description": "script data escaped dash and less-than paths",
        "input": "-- alpha < beta --></script>",
        "initial_state": "Script data escaped state",
        "last_start_tag": "script",
    },
    {
        "id": "script-data-double-escaped",
        "description": "script data double escaped text",
        "input": "alpha </script beta --></script>",
        "initial_state": "Script data double escaped state",
        "last_start_tag": "script",
    },
    {
        "id": "plaintext",
        "description": "PLAINTEXT consumes markup-looking input",
        "input": "alpha <p>&copy;</p>\u0000",
        "initial_state": "PLAINTEXT state",
    },
    {
        "id": "cdata-section",
        "description": "foreign-content CDATA section",
        "input": "alpha <p>&copy;</p>]]>tail",
        "initial_state": "CDATA section state",
    },
    {
        "id": "seeded-comment",
        "description": "seeded comment continuation",
        "input": "alpha--!>tail",
        "initial_state": "Comment state",
        "current_comment": "seed:",
    },
    {
        "id": "seeded-rcdata-end-tag-name",
        "description": "seeded RCDATA end tag-name continuation",
        "input": "le class=x></title>",
        "initial_state": "RCDATA end tag name state",
        "last_start_tag": "title",
        "current_end_tag": "tit",
        "temporary_buffer": "tit",
    },
    {
        "id": "seeded-rawtext-end-tag-attributes",
        "description": "seeded RAWTEXT end tag attributes continuation",
        "input": " class=x></style>",
        "initial_state": "RAWTEXT end tag attributes state",
        "last_start_tag": "style",
        "current_end_tag": "style",
        "temporary_buffer": "style",
    },
    {
        "id": "seeded-script-end-tag-whitespace",
        "description": "seeded script end-tag whitespace continuation",
        "input": " async></script>",
        "initial_state": "Script data end tag whitespace state",
        "last_start_tag": "script",
        "current_end_tag": "script",
        "temporary_buffer": "script",
    },
    {
        "id": "seeded-doctype-public-identifier",
        "description": "seeded DOCTYPE public identifier continuation",
        "input": 'alpha beta" "system">',
        "initial_state": "DOCTYPE public identifier double quoted state",
        "current_doctype": {
            "name": "html",
            "public_identifier": "seed:",
            "system_identifier": None,
            "force_quirks": False,
        },
    },
    {
        "id": "seeded-character-reference-data",
        "description": "seeded named character reference returning to data",
        "input": "opycat",
        "initial_state": "Named character reference state",
        "temporary_buffer": "&c",
        "return_state": "Data state",
    },
    {
        "id": "seeded-character-reference-rcdata",
        "description": "seeded numeric character reference returning to RCDATA",
        "input": "5;</title>",
        "initial_state": "Decimal character reference state",
        "last_start_tag": "title",
        "temporary_buffer": "6",
        "return_state": "RCDATA state",
    },
]


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    fixture = build_fixture()
    text = json.dumps(fixture, indent=2, ensure_ascii=False, sort_keys=True) + "\n"

    if args.check:
        existing = output.read_text()
        if existing != text:
            raise SystemExit(f"{output} is stale; regenerate it")
        return 0

    output.write_text(text)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer chunk-boundary fixture JSON."
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
    cases = []
    for case in CASES:
        materialized = dict(case)
        materialized["split_points"] = split_points(str(case["input"]))
        cases.append(materialized)
    return {
        "format": "whatwg-html-tokenizer-chunk-boundaries/v1",
        "description": (
            "Streaming chunk-boundary invariance cases for HTML tokenization "
            "contexts and seeded continuation states."
        ),
        "cases": cases,
    }


def split_points(value: str) -> list[int]:
    points = list(range(len(value) + 1))
    return points


if __name__ == "__main__":
    raise SystemExit(main())
