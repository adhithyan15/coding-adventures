#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer EOF recovery fixture.

The HTML tokenizer has many explicit EOF branches. This fixture keeps the
observable recovery shape pinned across ordinary tags, attributes, comments,
doctypes, character references, text modes, and seeded continuation states.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from generated_fixture_io import write_fixture_json


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-eof-recovery.json")

CASES = [
    {
        "id": "data-baseline",
        "description": "ordinary data EOF flushes text",
        "input": "alpha",
        "tokens": ["Text(data=alpha)", "EOF"],
    },
    {
        "id": "tag-open",
        "description": "EOF after a tag opener emits literal less-than",
        "input": "alpha<",
        "tokens": ["Text(data=alpha<)", "EOF"],
        "diagnostics": ["eof-in-tag-open-state"],
    },
    {
        "id": "end-tag-open",
        "description": "EOF after an end-tag opener emits literal markup",
        "input": "alpha</",
        "tokens": ["Text(data=alpha</)", "EOF"],
        "diagnostics": ["eof-in-end-tag-open-state"],
    },
    {
        "id": "start-tag-name",
        "description": "EOF inside a start tag name drops the partial token",
        "input": "<div",
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-tag-name-state"],
    },
    {
        "id": "start-tag-before-attribute",
        "description": "EOF before an attribute drops the partial token",
        "input": "<div ",
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-tag"],
    },
    {
        "id": "start-tag-attribute-name",
        "description": "EOF inside an attribute name drops the partial token",
        "input": "<div class",
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-tag"],
    },
    {
        "id": "start-tag-before-attribute-value",
        "description": "EOF before an attribute value drops the partial token",
        "input": "<div class=",
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-tag"],
    },
    {
        "id": "start-tag-double-quoted-attribute-value",
        "description": "EOF inside a double quoted attribute value drops the partial token",
        "input": '<div class="open',
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-tag"],
    },
    {
        "id": "start-tag-single-quoted-attribute-value",
        "description": "EOF inside a single quoted attribute value drops the partial token",
        "input": "<div class='open",
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-tag"],
    },
    {
        "id": "start-tag-unquoted-attribute-value",
        "description": "EOF inside an unquoted attribute value drops the partial token",
        "input": "<div class=open",
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-tag"],
    },
    {
        "id": "end-tag-name",
        "description": "EOF inside an end tag name drops the partial token",
        "input": "</section",
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-end-tag-name-state"],
    },
    {
        "id": "end-tag-with-attributes",
        "description": "EOF inside end-tag attributes drops the partial token",
        "input": "</section class=x",
        "tokens": ["EOF"],
        "diagnostics": ["end-tag-with-attributes", "eof-in-end-tag-name-state"],
    },
    {
        "id": "attribute-named-reference",
        "description": "EOF in an attribute named reference drops the partial tag",
        "input": "<a href=&copy",
        "tokens": ["EOF"],
        "diagnostics": ["missing-semicolon-after-character-reference", "eof-in-tag"],
    },
    {
        "id": "attribute-numeric-reference",
        "description": "EOF in an attribute numeric reference drops the partial tag",
        "input": "<a href=&#x41",
        "tokens": ["EOF"],
        "diagnostics": ["missing-semicolon-after-character-reference", "eof-in-tag"],
    },
    {
        "id": "attribute-digitless-numeric-reference",
        "description": "EOF in a digitless attribute numeric reference drops the partial tag",
        "input": "<a href=&#x",
        "tokens": ["EOF"],
        "diagnostics": ["absence-of-digits-in-numeric-character-reference", "eof-in-tag"],
    },
    {
        "id": "comment",
        "description": "EOF inside a comment emits the comment token",
        "input": "<!--open",
        "tokens": ["Comment(data=open)", "EOF"],
        "diagnostics": ["eof-in-comment"],
    },
    {
        "id": "comment-start",
        "description": "EOF after a comment opener emits an empty comment",
        "input": "<!--",
        "tokens": ["Comment(data=)", "EOF"],
        "diagnostics": ["eof-in-comment"],
    },
    {
        "id": "comment-start-dash",
        "description": "EOF after a pending comment dash emits an empty comment",
        "input": "<!---",
        "tokens": ["Comment(data=)", "EOF"],
        "diagnostics": ["eof-in-comment"],
    },
    {
        "id": "comment-end-dash",
        "description": "EOF after a trailing comment dash omits the delimiter dash",
        "input": "<!--x-",
        "tokens": ["Comment(data=x)", "EOF"],
        "diagnostics": ["eof-in-comment"],
    },
    {
        "id": "comment-end",
        "description": "EOF after trailing comment end dashes omits delimiter dashes",
        "input": "<!--x--",
        "tokens": ["Comment(data=x)", "EOF"],
        "diagnostics": ["eof-in-comment"],
    },
    {
        "id": "processing-instruction-target",
        "description": "EOF in a processing-instruction target discards the incomplete token",
        "input": "<?xml",
        "tokens": ["EOF"],
        "diagnostics": ["eof-in-processing-instruction"],
    },
    {
        "id": "markup-declaration-open",
        "description": "EOF after markup declaration opener recovers as empty comment",
        "input": "<!",
        "tokens": ["Comment(data=)", "EOF"],
        "diagnostics": ["incorrectly-opened-comment"],
    },
    {
        "id": "doctype-keyword",
        "description": "EOF inside the DOCTYPE keyword recovers as bogus comment",
        "input": "<!DOCT",
        "tokens": ["Comment(data=DOCT)", "EOF"],
        "diagnostics": ["incorrectly-opened-comment"],
    },
    {
        "id": "doctype-after-keyword",
        "description": "EOF after DOCTYPE keyword emits a forced-quirks doctype",
        "input": "<!DOCTYPE",
        "tokens": ["Doctype(name=null, force_quirks=true)", "EOF"],
        "diagnostics": ["eof-in-doctype"],
    },
    {
        "id": "doctype-name",
        "description": "EOF inside a DOCTYPE name emits a forced-quirks doctype",
        "input": "<!DOCTYPE html",
        "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
        "diagnostics": ["eof-in-doctype"],
    },
    {
        "id": "doctype-public-identifier",
        "description": "EOF inside a DOCTYPE public identifier emits a forced-quirks doctype",
        "input": '<!DOCTYPE html PUBLIC "alpha',
        "tokens": [
            "Doctype(name=html, public_identifier=alpha, system_identifier=null, force_quirks=true)",
            "EOF",
        ],
        "diagnostics": ["eof-in-doctype"],
    },
    {
        "id": "doctype-system-identifier",
        "description": "EOF inside a DOCTYPE system identifier emits a forced-quirks doctype",
        "input": '<!DOCTYPE html SYSTEM "alpha',
        "tokens": [
            "Doctype(name=html, public_identifier=null, system_identifier=alpha, force_quirks=true)",
            "EOF",
        ],
        "diagnostics": ["eof-in-doctype"],
    },
    {
        "id": "data-named-reference",
        "description": "EOF after a legacy named reference recovers in data",
        "input": "alpha &copy",
        "tokens": ["Text(data=alpha ©)", "EOF"],
        "diagnostics": ["missing-semicolon-after-character-reference"],
    },
    {
        "id": "data-unknown-reference",
        "description": "EOF after an unknown named reference stays literal",
        "input": "alpha &madeup",
        "tokens": ["Text(data=alpha &madeup)", "EOF"],
    },
    {
        "id": "data-digitless-numeric-reference",
        "description": "EOF after a digitless numeric reference stays literal",
        "input": "alpha &#x",
        "tokens": ["Text(data=alpha &#x)", "EOF"],
        "diagnostics": ["absence-of-digits-in-numeric-character-reference"],
    },
    {
        "id": "rcdata-less-than-sign",
        "description": "EOF after less-than in RCDATA emits literal less-than",
        "input": "alpha<",
        "initial_state": "RCDATA state",
        "last_start_tag": "title",
        "tokens": ["Text(data=alpha<)", "EOF"],
    },
    {
        "id": "rcdata-end-tag-open",
        "description": "EOF after RCDATA end-tag opener emits literal markup",
        "input": "alpha</",
        "initial_state": "RCDATA state",
        "last_start_tag": "title",
        "tokens": ["Text(data=alpha</)", "EOF"],
    },
    {
        "id": "rawtext-end-tag-open",
        "description": "EOF after RAWTEXT end-tag opener emits literal markup",
        "input": "alpha</",
        "initial_state": "RAWTEXT state",
        "last_start_tag": "style",
        "tokens": ["Text(data=alpha</)", "EOF"],
    },
    {
        "id": "script-data-less-than-sign",
        "description": "EOF after script less-than emits literal less-than",
        "input": "alpha<",
        "initial_state": "Script data state",
        "last_start_tag": "script",
        "tokens": ["Text(data=alpha<)", "EOF"],
    },
    {
        "id": "script-data-escaped",
        "description": "EOF inside script escaped text reports script comment-like EOF",
        "input": "alpha",
        "initial_state": "Script data escaped state",
        "last_start_tag": "script",
        "tokens": ["Text(data=alpha)", "EOF"],
        "diagnostics": ["eof-in-script-html-comment-like-text"],
    },
    {
        "id": "script-data-double-escaped",
        "description": "EOF inside script double escaped text reports script comment-like EOF",
        "input": "alpha",
        "initial_state": "Script data double escaped state",
        "last_start_tag": "script",
        "tokens": ["Text(data=alpha)", "EOF"],
        "diagnostics": ["eof-in-script-html-comment-like-text"],
    },
    {
        "id": "plaintext",
        "description": "EOF in PLAINTEXT flushes markup-looking text",
        "input": "alpha <p>&copy;",
        "initial_state": "PLAINTEXT state",
        "tokens": ["Text(data=alpha <p>&copy;)", "EOF"],
    },
    {
        "id": "cdata-section-bracket",
        "description": "EOF with unclosed CDATA section brackets keeps brackets",
        "input": "alpha]]",
        "initial_state": "CDATA section state",
        "tokens": ["Text(data=alpha]])", "EOF"],
    },
    {
        "id": "seeded-comment",
        "description": "EOF in a seeded comment emits seeded data",
        "input": "tail",
        "initial_state": "Comment state",
        "current_comment": "seed:",
        "tokens": ["Comment(data=seed:tail)", "EOF"],
        "diagnostics": ["eof-in-comment"],
    },
    {
        "id": "seeded-doctype-public-identifier",
        "description": "EOF in a seeded DOCTYPE public identifier preserves seed",
        "input": "tail",
        "initial_state": "DOCTYPE public identifier double quoted state",
        "current_doctype": {
            "name": "html",
            "public_identifier": "seed:",
            "system_identifier": None,
            "force_quirks": False,
        },
        "tokens": [
            "Doctype(name=html, public_identifier=seed:tail, system_identifier=null, force_quirks=true)",
            "EOF",
        ],
        "diagnostics": ["eof-in-doctype"],
    },
    {
        "id": "seeded-character-reference-rcdata",
        "description": "EOF in a seeded RCDATA named reference recovers through return state",
        "input": "opy",
        "initial_state": "Named character reference state",
        "last_start_tag": "title",
        "temporary_buffer": "&c",
        "return_state": "RCDATA state",
        "tokens": ["Text(data=©)", "EOF"],
        "diagnostics": ["missing-semicolon-after-character-reference"],
    },
]


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    fixture = build_fixture()
    return write_fixture_json(output, fixture, check=args.check)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer EOF recovery fixture JSON."
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
        "format": "whatwg-html-tokenizer-eof-recovery/v1",
        "description": (
            "EOF recovery cases for HTML tokenizer states and parser-seeded "
            "continuation contexts."
        ),
        "cases": [normalize_case(case) for case in CASES],
    }


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
