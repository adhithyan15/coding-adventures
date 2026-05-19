#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer DOCTYPE boundary fixture.

The broader markup-declaration fixture proves that `<!DOCTYPE` dispatch works.
This focused suite pins the boundary states after that dispatch: keyword/name
whitespace, PUBLIC/SYSTEM identifier selection, quoted identifier recovery,
NULL replacement, force-quirks handling, and seeded continuation contexts.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from generated_fixture_io import write_fixture_json


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-doctype-boundaries.json")


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    fixture = build_fixture()
    return write_fixture_json(output, fixture, check=args.check)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer DOCTYPE boundary fixture JSON."
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
        "format": "whatwg-html-tokenizer-doctype-boundaries/v1",
        "description": (
            "DOCTYPE keyword, name, PUBLIC/SYSTEM identifier, force-quirks, "
            "NULL replacement, EOF, and seeded continuation boundary recovery."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = []
    cases.extend(name_boundary_cases())
    cases.extend(public_identifier_cases())
    cases.extend(system_identifier_cases())
    cases.extend(eof_and_bogus_cases())
    cases.extend(seeded_boundary_cases())
    return cases


def name_boundary_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "doctype-name-tab",
            "description": "tab after DOCTYPE keyword begins the name",
            "input": "<!DOCTYPE\thtml>",
            "tokens": ["Doctype(name=html, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-name-line-feed",
            "description": "line feed after DOCTYPE keyword begins the name",
            "input": "<!DOCTYPE\nhtml>",
            "tokens": ["Doctype(name=html, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-name-form-feed",
            "description": "form feed after DOCTYPE keyword begins the name",
            "input": "<!DOCTYPE\u000chtml>",
            "tokens": ["Doctype(name=html, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-name-leading-whitespace-run",
            "description": "multiple HTML whitespace characters before the name are skipped",
            "input": "<!DOCTYPE \t\n\u000cHTML>",
            "tokens": ["Doctype(name=html, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-name-uppercase-mixed",
            "description": "DOCTYPE names are ASCII-lowercased while reading the name",
            "input": "<!DOCTYPE HtMl>",
            "tokens": ["Doctype(name=html, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-name-null-replacement",
            "description": "NULL inside a DOCTYPE name becomes U+FFFD",
            "input": "<!DOCTYPE h\u0000tml>",
            "tokens": ["Doctype(name=h�tml, force_quirks=false)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "doctype-missing-name-before-comment",
            "description": "hyphen text after DOCTYPE keyword is accepted as the name",
            "input": "<!DOCTYPE --html>",
            "tokens": ["Doctype(name=--html, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-name-missing-whitespace-before-public",
            "description": "PUBLIC text without whitespace remains part of the name",
            "input": "<!DOCTYPE htmlPUBLIC>",
            "tokens": ["Doctype(name=htmlpublic, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-name-missing-whitespace-before-system",
            "description": "SYSTEM text without whitespace remains part of the name",
            "input": "<!DOCTYPE htmlSYSTEM>",
            "tokens": ["Doctype(name=htmlsystem, force_quirks=false)", "EOF"],
        },
        {
            "id": "doctype-name-public-prefix-not-keyword",
            "description": "non-keyword text after a name emits the current name",
            "input": "<!DOCTYPE html PUBLIK>",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["invalid-character-sequence-after-doctype-name"],
        },
    ]


def public_identifier_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "doctype-public-tab-before-keyword",
            "description": "tab separates the name from a PUBLIC keyword",
            "input": '<!DOCTYPE html\tPUBLIC "pub">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-line-feed-before-keyword",
            "description": "line feed separates the name from a PUBLIC keyword",
            "input": '<!DOCTYPE html\nPUBLIC "pub">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-form-feed-before-keyword",
            "description": "form feed separates the name from a PUBLIC keyword",
            "input": '<!DOCTYPE html\u000cPUBLIC "pub">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-lowercase-keyword",
            "description": "PUBLIC keyword matching is ASCII-case-insensitive",
            "input": '<!DOCTYPE html public "pub">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-tab-before-identifier",
            "description": "tab after PUBLIC keyword can precede the identifier quote",
            "input": '<!DOCTYPE html PUBLIC\t"pub">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-line-feed-before-identifier",
            "description": "line feed after PUBLIC keyword can precede the identifier quote",
            "input": '<!DOCTYPE html PUBLIC\n"pub">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-single-quoted-identifier",
            "description": "single-quoted PUBLIC identifiers are preserved",
            "input": "<!DOCTYPE html PUBLIC 'pub'>",
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-empty-identifier",
            "description": "empty PUBLIC identifiers stay empty and do not force quirks",
            "input": '<!DOCTYPE html PUBLIC "">',
            "tokens": [
                "Doctype(name=html, public_identifier=, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-null-in-identifier",
            "description": "NULL inside a PUBLIC identifier becomes U+FFFD",
            "input": '<!DOCTYPE html PUBLIC "a\u0000b">',
            "tokens": [
                "Doctype(name=html, public_identifier=a�b, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "doctype-public-missing-whitespace-before-identifier",
            "description": "a quote immediately after PUBLIC is recovered as the identifier",
            "input": '<!DOCTYPE html PUBLIC"pub">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["missing-whitespace-after-doctype-public-keyword"],
        },
        {
            "id": "doctype-public-missing-quote",
            "description": "PUBLIC without an identifier quote forces quirks mode",
            "input": "<!DOCTYPE html PUBLIC pub>",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["missing-quote-before-doctype-public-identifier"],
        },
        {
            "id": "doctype-public-abrupt-close",
            "description": "PUBLIC keyword closed before an identifier forces quirks mode",
            "input": "<!DOCTYPE html PUBLIC>",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["missing-doctype-public-identifier"],
        },
        {
            "id": "doctype-public-system-with-space",
            "description": "a system identifier after PUBLIC is read after whitespace",
            "input": '<!DOCTYPE html PUBLIC "pub" "sys">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-system-with-tab",
            "description": "tab separates PUBLIC and system identifiers",
            "input": '<!DOCTYPE html PUBLIC "pub"\t"sys">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-public-system-missing-whitespace",
            "description": "a system quote immediately after PUBLIC identifier is recovered",
            "input": '<!DOCTYPE html PUBLIC "pub""sys">',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["missing-whitespace-between-doctype-public-and-system-identifiers"],
        },
        {
            "id": "doctype-public-trailing-junk",
            "description": "text after a PUBLIC identifier is recovered as a missing system quote",
            "input": '<!DOCTYPE html PUBLIC "pub" junk>',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=true)",
                "EOF",
            ],
            "diagnostics": ["missing-quote-before-doctype-system-identifier"],
        },
    ]


def system_identifier_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "doctype-system-tab-before-keyword",
            "description": "tab separates the name from a SYSTEM keyword",
            "input": '<!DOCTYPE html\tSYSTEM "sys">',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-system-lowercase-keyword",
            "description": "SYSTEM keyword matching is ASCII-case-insensitive",
            "input": '<!DOCTYPE html system "sys">',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-system-form-feed-before-identifier",
            "description": "form feed after SYSTEM keyword can precede the identifier quote",
            "input": '<!DOCTYPE html SYSTEM\u000c"sys">',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-system-single-quoted-identifier",
            "description": "single-quoted SYSTEM identifiers are preserved",
            "input": "<!DOCTYPE html SYSTEM 'sys'>",
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-system-empty-identifier",
            "description": "empty SYSTEM identifiers stay empty and do not force quirks",
            "input": '<!DOCTYPE html SYSTEM "">',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "doctype-system-null-in-identifier",
            "description": "NULL inside a SYSTEM identifier becomes U+FFFD",
            "input": '<!DOCTYPE html SYSTEM "s\u0000s">',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=s�s, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "doctype-system-missing-whitespace-before-identifier",
            "description": "a quote immediately after SYSTEM is recovered as the identifier",
            "input": '<!DOCTYPE html SYSTEM"sys">',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["missing-whitespace-after-doctype-system-keyword"],
        },
        {
            "id": "doctype-system-missing-quote",
            "description": "SYSTEM without an identifier quote forces quirks mode",
            "input": "<!DOCTYPE html SYSTEM sys>",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["missing-quote-before-doctype-system-identifier"],
        },
        {
            "id": "doctype-system-abrupt-close",
            "description": "SYSTEM keyword closed before an identifier forces quirks mode",
            "input": "<!DOCTYPE html SYSTEM>",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["missing-doctype-system-identifier"],
        },
        {
            "id": "doctype-system-trailing-junk",
            "description": "unexpected text after a SYSTEM identifier is diagnosed",
            "input": '<!DOCTYPE html SYSTEM "sys" junk>',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-character-after-doctype-system-identifier"],
        },
    ]


def eof_and_bogus_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "doctype-eof-after-keyword",
            "description": "EOF immediately after DOCTYPE keyword emits force-quirks",
            "input": "<!DOCTYPE",
            "tokens": ["Doctype(name=null, force_quirks=true)", "EOF"],
            "diagnostics": ["eof-in-doctype"],
        },
        {
            "id": "doctype-eof-after-name",
            "description": "EOF inside a DOCTYPE name emits force-quirks",
            "input": "<!DOCTYPE html",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["eof-in-doctype"],
        },
        {
            "id": "doctype-eof-after-public-keyword",
            "description": "EOF after PUBLIC keyword emits force-quirks",
            "input": "<!DOCTYPE html PUBLIC",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["eof-in-doctype"],
        },
        {
            "id": "doctype-eof-in-public-identifier",
            "description": "EOF inside a PUBLIC identifier emits force-quirks",
            "input": '<!DOCTYPE html PUBLIC "pub',
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=true)",
                "EOF",
            ],
            "diagnostics": ["eof-in-doctype"],
        },
        {
            "id": "doctype-eof-after-system-keyword",
            "description": "EOF after SYSTEM keyword emits force-quirks",
            "input": "<!DOCTYPE html SYSTEM",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["eof-in-doctype"],
        },
        {
            "id": "doctype-eof-in-system-identifier",
            "description": "EOF inside a SYSTEM identifier emits force-quirks",
            "input": '<!DOCTYPE html SYSTEM "sys',
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=true)",
                "EOF",
            ],
            "diagnostics": ["eof-in-doctype"],
        },
        {
            "id": "doctype-bogus-null-discard",
            "description": "bogus DOCTYPE recovery discards NULL after reporting it",
            "input": "<!DOCTYPE html PUBLIC pub\u0000rest>",
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": [
                "missing-quote-before-doctype-public-identifier",
                "unexpected-null-character",
            ],
        },
    ]


def seeded_boundary_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "seeded-after-keyword-whitespace-to-name",
            "description": "seeded after-keyword state skips whitespace before the name",
            "input": "\thtml>",
            "initial_state": "Before DOCTYPE name state",
            "current_doctype": {},
            "tokens": ["Doctype(name=html, force_quirks=false)", "EOF"],
        },
        {
            "id": "seeded-name-to-public-keyword",
            "description": "seeded after-name state recognizes a following PUBLIC keyword",
            "input": ' PUBLIC "pub">',
            "initial_state": "After DOCTYPE name state",
            "current_doctype": {"name": "html"},
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "seeded-name-to-system-keyword",
            "description": "seeded after-name state recognizes a following SYSTEM keyword",
            "input": ' SYSTEM "sys">',
            "initial_state": "After DOCTYPE name state",
            "current_doctype": {"name": "html"},
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "seeded-after-public-keyword-missing-whitespace",
            "description": "seeded after-PUBLIC state recovers a quote without whitespace",
            "input": '"pub">',
            "initial_state": "After DOCTYPE public keyword state",
            "current_doctype": {"name": "html"},
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["missing-whitespace-after-doctype-public-keyword"],
        },
        {
            "id": "seeded-public-identifier-single-quoted-null",
            "description": "seeded single-quoted PUBLIC identifiers replace NULL",
            "input": "\u0000id'>",
            "initial_state": "DOCTYPE public identifier single quoted state",
            "current_doctype": {"name": "html", "public_identifier": "pub-"},
            "tokens": [
                "Doctype(name=html, public_identifier=pub-�id, system_identifier=null, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "seeded-between-identifiers-space-to-system",
            "description": "seeded between-identifiers state skips whitespace before system quote",
            "input": ' \t"sys">',
            "initial_state": "Between DOCTYPE public and system identifiers state",
            "current_doctype": {"name": "html", "public_identifier": "pub"},
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
        },
        {
            "id": "seeded-after-system-keyword-missing-whitespace",
            "description": "seeded after-SYSTEM state recovers a quote without whitespace",
            "input": '"sys">',
            "initial_state": "After DOCTYPE system keyword state",
            "current_doctype": {"name": "html"},
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["missing-whitespace-after-doctype-system-keyword"],
        },
        {
            "id": "seeded-system-identifier-single-quoted-null",
            "description": "seeded single-quoted SYSTEM identifiers replace NULL",
            "input": "\u0000id'>",
            "initial_state": "DOCTYPE system identifier single quoted state",
            "current_doctype": {"name": "html", "system_identifier": "sys-"},
            "tokens": [
                "Doctype(name=html, public_identifier=null, system_identifier=sys-�id, force_quirks=false)",
                "EOF",
            ],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "seeded-after-public-identifier-trailing-junk",
            "description": "seeded after-PUBLIC-identifier state recovers text as a missing system quote",
            "input": " junk>",
            "initial_state": "After DOCTYPE public identifier state",
            "current_doctype": {"name": "html", "public_identifier": "pub"},
            "tokens": [
                "Doctype(name=html, public_identifier=pub, system_identifier=null, force_quirks=true)",
                "EOF",
            ],
            "diagnostics": ["missing-quote-before-doctype-system-identifier"],
        },
        {
            "id": "seeded-bogus-doctype-null-discard",
            "description": "seeded bogus DOCTYPE recovery discards NULL after reporting it",
            "input": "\u0000ignored>",
            "initial_state": "Bogus DOCTYPE state",
            "current_doctype": {"name": "html", "force_quirks": True},
            "tokens": ["Doctype(name=html, force_quirks=true)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
    ]


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
