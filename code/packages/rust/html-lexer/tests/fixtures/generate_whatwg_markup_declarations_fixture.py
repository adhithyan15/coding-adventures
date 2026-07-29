#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer markup-declaration fixture.

Markup declaration tokenization fans out from `<!` into comments, DOCTYPE,
CDATA, and bogus-comment recovery. This fixture pins that visible surface across
ordinary data-state inputs plus seeded continuation states that parser and
html5lib-style adapters can resume through the typed Rust context API.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from generated_fixture_io import write_fixture_json


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-markup-declarations.json")


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    fixture = build_fixture()
    return write_fixture_json(output, fixture, check=args.check)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer markup-declaration fixture JSON."
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
        "format": "whatwg-html-tokenizer-markup-declarations/v1",
        "description": (
            "Markup declaration recovery for comments, bogus comments, CDATA, "
            "DOCTYPE tokens, and seeded continuation states."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = []
    cases.extend(comment_cases())
    cases.extend(bogus_comment_cases())
    cases.extend(cdata_cases())
    cases.extend(doctype_cases())
    cases.extend(seeded_comment_cases())
    cases.extend(seeded_cdata_cases())
    cases.extend(seeded_doctype_cases())
    return cases


def comment_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "comment-basic",
            "description": "markup declaration recognizes an ordinary HTML comment",
            "input": "a<!--note-->b",
            "tokens": ["Text(data=a)", "Comment(data=note)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-empty-abrupt",
            "description": "empty comment closes through the abrupt <!--> recovery path",
            "input": "a<!-->b",
            "tokens": ["Text(data=a)", "Comment(data=)", "Text(data=b)", "EOF"],
            "diagnostics": ["abrupt-closing-of-empty-comment"],
        },
        {
            "id": "comment-empty-start-dash",
            "description": "three-dash empty comment closes without comment text",
            "input": "a<!--->b",
            "tokens": ["Text(data=a)", "Comment(data=)", "Text(data=b)", "EOF"],
            "diagnostics": ["abrupt-closing-of-empty-comment"],
        },
        {
            "id": "comment-nested-looking-opener",
            "description": "nested-looking comment openers stay literal comment data",
            "input": "a<!--x<!--y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x<!--y)", "Text(data=b)", "EOF"],
            "diagnostics": ["nested-comment"],
        },
        {
            "id": "comment-end-bang-close",
            "description": "incorrect --!> comment close emits the comment and diagnostic",
            "input": "a<!--x--!>b",
            "tokens": ["Text(data=a)", "Comment(data=x)", "Text(data=b)", "EOF"],
            "diagnostics": ["incorrectly-closed-comment"],
        },
        {
            "id": "comment-end-bang-not-close",
            "description": "non-closing --! text remains inside the comment",
            "input": "a<!--x--!y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x--!y)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-null-replacement",
            "description": "NULL inside comment data is replaced with U+FFFD",
            "input": "a<!--x\u0000y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x�y)", "Text(data=b)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "comment-less-than-text",
            "description": "less-than signs that do not nest remain comment text",
            "input": "a<!--x<y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x<y)", "Text(data=b)", "EOF"],
        },
    ]


def bogus_comment_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "disallowed-processing-instruction-target-comment",
            "description": "the reserved xml processing-instruction target recovers as a bogus comment",
            "input": "a<?xml version='1.0'?>b",
            "tokens": [
                "Text(data=a)",
                "Comment(data=?xml version='1.0'?)",
                "Text(data=b)",
                "EOF",
            ],
            "diagnostics": ["disallowed-processing-instruction-target"],
        },
        {
            "id": "malformed-declaration-bogus-comment",
            "description": "unknown markup declarations recover as bogus comments",
            "input": "a<!foo>b",
            "tokens": ["Text(data=a)", "Comment(data=foo)", "Text(data=b)", "EOF"],
            "diagnostics": ["incorrectly-opened-comment"],
        },
        {
            "id": "empty-malformed-declaration",
            "description": "empty malformed declaration emits an empty bogus comment",
            "input": "a<!>b",
            "tokens": ["Text(data=a)", "Comment(data=)", "Text(data=b)", "EOF"],
            "diagnostics": ["incorrectly-opened-comment"],
        },
        {
            "id": "one-dash-declaration",
            "description": "one-dash markup declaration uses bogus-comment recovery",
            "input": "a<!-x>b",
            "tokens": ["Text(data=a)", "Comment(data=-x)", "Text(data=b)", "EOF"],
            "diagnostics": ["incorrectly-opened-comment"],
        },
        {
            "id": "invalid-end-tag-open-bogus-comment",
            "description": "malformed end-tag openers recover as bogus comments",
            "input": "a</3>b",
            "tokens": ["Text(data=a)", "Comment(data=3)", "Text(data=b)", "EOF"],
            "diagnostics": ["invalid-first-character-of-tag-name"],
        },
        {
            "id": "bogus-comment-null-replacement",
            "description": "NULL inside bogus comments is replaced before emission",
            "input": "a<!foo\u0000bar>b",
            "tokens": ["Text(data=a)", "Comment(data=foo�bar)", "Text(data=b)", "EOF"],
            "diagnostics": ["incorrectly-opened-comment", "unexpected-null-character"],
        },
        {
            "id": "malformed-cdata-bogus-comment",
            "description": "partial CDATA openers recover as bogus comments",
            "input": "a<![CDATAx]>b",
            "tokens": ["Text(data=a)", "Comment(data=[CDATAx])", "Text(data=b)", "EOF"],
        },
    ]


def cdata_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "cdata-section-from-markup-declaration",
            "description": "default HTML data state recovers CDATA opener as a bogus comment",
            "input": "a<![CDATA[x<y&z]]>b",
            "tokens": [
                "Text(data=a)",
                "Comment(data=[CDATA[x<y&z]])",
                "Text(data=b)",
                "EOF",
            ],
            "diagnostics": ["cdata-in-html-content"],
        },
        {
            "id": "cdata-section-bracket-not-end",
            "description": "default HTML CDATA-looking declarations stay bogus comments",
            "input": "a<![CDATA[x]y]]>b",
            "tokens": [
                "Text(data=a)",
                "Comment(data=[CDATA[x]y]])",
                "Text(data=b)",
                "EOF",
            ],
            "diagnostics": ["cdata-in-html-content"],
        },
        {
            "id": "cdata-section-null-replacement",
            "description": "NULL inside default HTML CDATA-looking bogus comments is replaced",
            "input": "a<![CDATA[x\u0000y]]>b",
            "tokens": [
                "Text(data=a)",
                "Comment(data=[CDATA[x�y]])",
                "Text(data=b)",
                "EOF",
            ],
            "diagnostics": ["cdata-in-html-content", "unexpected-null-character"],
        },
    ]


def doctype_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "doctype-simple",
            "description": "simple DOCTYPE emits a non-quirks doctype token",
            "input": "<!DOCTYPE html>",
            "tokens": ["Doctype(name=html, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-uppercase-keyword-and-name",
            "description": "DOCTYPE keyword and name are ASCII-case-insensitive",
            "input": "<!doctype HTML>",
            "tokens": ["Doctype(name=html, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-missing-name",
            "description": "DOCTYPE without a name forces quirks mode",
            "input": "<!DOCTYPE>",
            "tokens": ["Doctype(name=null, force_quirks=true)", "EOF"],
            "diagnostics": ["missing-doctype-name"],
        },
        {
            "id": "doctype-missing-name-after-whitespace",
            "description": "DOCTYPE with only whitespace before close forces quirks mode",
            "input": "<!DOCTYPE >",
            "tokens": ["Doctype(name=null, force_quirks=true)", "EOF"],
            "diagnostics": ["missing-doctype-name"],
        },
        {
            "id": "doctype-public-identifier",
            "description": "PUBLIC identifier is preserved on the DOCTYPE token",
            "input": '<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN">',
            "tokens": [
                "Doctype(name=html, public_identifier=-//W3C//DTD HTML 4.01//EN, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-and-system-identifiers",
            "description": "PUBLIC and system identifiers are preserved together",
            "input": '<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN" "about:legacy-compat">',
            "tokens": [
                "Doctype(name=html, public_identifier=-//W3C//DTD HTML 4.01//EN, system_identifier=about:legacy-compat, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-system-identifier",
            "description": "SYSTEM identifier is preserved on the DOCTYPE token",
            "input": '<!DOCTYPE html SYSTEM "about:legacy-compat">',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=about:legacy-compat, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-single-quoted-identifiers",
            "description": "single-quoted PUBLIC and system identifiers are preserved",
            "input": "<!DOCTYPE html PUBLIC '-//ID//' 'sys'>",
            "tokens": [
                "Doctype(name=html, public_identifier=-//ID//, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-null-in-name",
            "description": "NULL in a DOCTYPE name is replaced with U+FFFD",
            "input": "<!DOCTYPE ht\u0000ml>",
            "tokens": ["Doctype(name=ht�ml, force_quirks=false)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "doctype-null-in-public-identifier",
            "description": "NULL in a public identifier is replaced with U+FFFD",
            "input": '<!DOCTYPE html PUBLIC "a\u0000b">',
            "tokens": [
                "Doctype(name=html, public_identifier=a�b, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "doctype-missing-whitespace-before-public",
            "description": "PUBLIC keyword without separating whitespace is diagnosed",
            "input": "<!DOCTYPE htmlPUBLIC>",
            "tokens": ["Doctype(name=htmlpublic, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-missing-public-identifier-quotes",
            "description": "PUBLIC without quoted identifier forces quirks mode",
            "input": "<!DOCTYPE html PUBLIC x>",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["missing-quote-before-doctype-public-identifier"],
        },
        {
            "id": "doctype-missing-system-identifier-quotes",
            "description": "SYSTEM without quoted identifier forces quirks mode",
            "input": "<!DOCTYPE html SYSTEM x>",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["missing-quote-before-doctype-system-identifier"],
        },
        {
            "id": "doctype-unexpected-trailing-junk",
            "description": "trailing junk after a system identifier is diagnosed without forcing quirks",
            "input": '<!DOCTYPE html SYSTEM "sys" junk>',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-character-after-doctype-system-identifier"],
        },
        {
            "id": "doctype-malformed-keyword",
            "description": "malformed DOCTYPE keyword recovers as a bogus comment",
            "input": "<!DOCTYPX html>",
            "tokens": ["Comment(data=DOCTYPX html)", "EOF"],
            "diagnostics": ["incorrectly-opened-comment"],
        },
    ]


def seeded_comment_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "seeded-comment-body-close",
            "description": "seeded comment body appends data before closing",
            "input": " tail-->after",
            "initial_state": "Comment state",
            "current_comment": "seed",
            "tokens": ["Comment(data=seed tail)", "Text(data=after)", "EOF"],
        },
        {
            "id": "seeded-comment-start-dash-close",
            "description": "seeded comment start dash can close an empty comment",
            "input": ">after",
            "initial_state": "Comment start dash state",
            "current_comment": "",
            "tokens": ["Comment(data=)", "Text(data=after)", "EOF"],
            "diagnostics": ["abrupt-closing-of-empty-comment"],
        },
        {
            "id": "seeded-comment-end-dash-close",
            "description": "seeded comment end dash preserves pending text before close",
            "input": "->after",
            "initial_state": "Comment end dash state",
            "current_comment": "seed",
            "tokens": ["Comment(data=seed)", "Text(data=after)", "EOF"],
        },
        {
            "id": "seeded-comment-end-bang-close",
            "description": "seeded comment end bang closes with incorrectly-closed diagnostic",
            "input": ">after",
            "initial_state": "Comment end bang state",
            "current_comment": "seed",
            "tokens": ["Comment(data=seed)", "Text(data=after)", "EOF"],
            "diagnostics": ["incorrectly-closed-comment"],
        },
        {
            "id": "seeded-bogus-comment-close",
            "description": "seeded bogus comment closes on greater-than",
            "input": " tail>after",
            "initial_state": "Bogus comment state",
            "current_comment": "seed",
            "tokens": ["Comment(data=seed tail)", "Text(data=after)", "EOF"],
        },
    ]


def seeded_cdata_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "seeded-cdata-section-close",
            "description": "seeded CDATA section returns to data state at delimiter",
            "input": "x]]>after",
            "initial_state": "CDATA section state",
            "tokens": ["Text(data=x)", "Text(data=after)", "EOF"],
        },
        {
            "id": "seeded-cdata-bracket-not-close",
            "description": "seeded CDATA bracket substate preserves non-closing bracket",
            "input": "x]]>after",
            "initial_state": "CDATA section bracket state",
            "tokens": ["Text(data=]x)", "Text(data=after)", "EOF"],
        },
        {
            "id": "seeded-cdata-end-close",
            "description": "seeded CDATA end substate closes on greater-than",
            "input": ">after",
            "initial_state": "CDATA section end state",
            "tokens": ["Text(data=after)", "EOF"],
        },
    ]


def seeded_doctype_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "seeded-doctype-name-close",
            "description": "seeded DOCTYPE name closes as a non-quirks token",
            "input": ">after",
            "initial_state": "DOCTYPE name state",
            "current_doctype": {"name": "html"},
            "tokens": [
                "Doctype(name=html, force_quirks=false)",
                "Text(data=after)",
                "EOF",
            ],
        },
        {
            "id": "seeded-doctype-name-null",
            "description": "seeded DOCTYPE name state replaces NULL while appending",
            "input": "\u0000>",
            "initial_state": "DOCTYPE name state",
            "current_doctype": {"name": "ht"},
            "tokens": ["Doctype(name=ht�, force_quirks=false)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "seeded-public-identifier-close",
            "description": "seeded public identifier appends text before closing",
            "input": 'id">',
            "initial_state": "DOCTYPE public identifier double quoted state",
            "current_doctype": {"name": "html", "public_identifier": "pub-"},
            "tokens": [
                "Doctype(name=html, public_identifier=pub-id, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "seeded-between-public-and-system",
            "description": "seeded between identifiers reads a following system identifier",
            "input": '"sys">after',
            "initial_state": "Between DOCTYPE public and system identifiers state",
            "current_doctype": {"name": "html", "public_identifier": "pub"},
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=sys, force_quirks=false)",
                "Text(data=after)",
                "EOF",
            ],
        },
        {
            "id": "seeded-system-identifier-null",
            "description": "seeded system identifier replaces NULL while appending",
            "input": '\u0000">',
            "initial_state": "DOCTYPE system identifier double quoted state",
            "current_doctype": {"name": "html", "system_identifier": "sys"},
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys�, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "seeded-after-system-identifier-junk",
            "description": "seeded after-system-identifier state diagnoses trailing junk",
            "input": " junk>",
            "initial_state": "After DOCTYPE system identifier state",
            "current_doctype": {"name": "html", "system_identifier": "sys"},
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-character-after-doctype-system-identifier"],
        },
        {
            "id": "seeded-bogus-doctype-close",
            "description": "seeded bogus DOCTYPE closes while preserving force-quirks",
            "input": " ignored>",
            "initial_state": "Bogus DOCTYPE state",
            "current_doctype": {"name": "html", "force_quirks": True},
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
        },
    ]


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
