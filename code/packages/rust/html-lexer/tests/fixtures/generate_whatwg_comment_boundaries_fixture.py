#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer comment boundary fixture.

The broader markup-declaration fixture proves that `<!--` and malformed `<!`
dispatch into comment recovery. This focused suite pins the states after that
dispatch: ordinary comments, less-than/bang nested-comment recovery, end-dash
and end-bang handling, bogus comments, NULL replacement, EOF recovery, and
seeded parser continuation contexts.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from generated_fixture_io import write_fixture_json


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-comment-boundaries.json")


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    fixture = build_fixture()
    return write_fixture_json(output, fixture, check=args.check)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer comment boundary fixture JSON."
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
        "format": "whatwg-html-tokenizer-comment-boundaries/v1",
        "description": (
            "Comment, bogus-comment, nested-comment, EOF, NULL replacement, "
            "end-bang, and seeded continuation boundary recovery."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = []
    cases.extend(comment_data_cases())
    cases.extend(comment_eof_cases())
    cases.extend(bogus_comment_cases())
    cases.extend(seeded_comment_cases())
    cases.extend(seeded_bogus_comment_cases())
    return cases


def comment_data_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "comment-basic-close",
            "description": "ordinary comments emit their data between opener and close",
            "input": "a<!--note-->b",
            "tokens": ["Text(data=a)", "Comment(data=note)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-empty-abrupt",
            "description": "empty comments may close abruptly from comment start state",
            "input": "a<!-->b",
            "tokens": ["Text(data=a)", "Comment(data=)", "Text(data=b)", "EOF"],
            "diagnostics": ["abrupt-closing-of-empty-comment"],
        },
        {
            "id": "comment-empty-start-dash",
            "description": "comment start dash state closes abruptly on greater-than",
            "input": "a<!--->b",
            "tokens": ["Text(data=a)", "Comment(data=)", "Text(data=b)", "EOF"],
            "diagnostics": ["abrupt-closing-of-empty-comment"],
        },
        {
            "id": "comment-double-dash-empty",
            "description": "the shortest non-abrupt comment close emits empty data",
            "input": "a<!---->b",
            "tokens": ["Text(data=a)", "Comment(data=)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-null-body",
            "description": "NULL inside comment data is replaced with U+FFFD",
            "input": "a<!--x\u0000y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x�y)", "Text(data=b)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "comment-null-after-start",
            "description": "NULL immediately after the opener is replaced in comment start state",
            "input": "a<!--\u0000-->b",
            "tokens": ["Text(data=a)", "Comment(data=�)", "Text(data=b)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "comment-null-after-start-dash",
            "description": "NULL after one pending dash preserves the dash before replacement",
            "input": "a<!---\u0000-->b",
            "tokens": ["Text(data=a)", "Comment(data=-�)", "Text(data=b)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "comment-less-than-text",
            "description": "less-than signs that do not introduce nested comments stay literal",
            "input": "a<!--x<y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x<y)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-less-than-repeat",
            "description": "repeated less-than signs remain in the comment text",
            "input": "a<!--x<<y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x<<y)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-less-than-bang-text",
            "description": "less-than bang without following dashes stays literal",
            "input": "a<!--x<!y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x<!y)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-less-than-bang-dash-text",
            "description": "less-than bang dash without a second dash stays literal",
            "input": "a<!--x<!-y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x<!-y)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-nested-looking-opener",
            "description": "nested-looking comment openers stay literal and report nested-comment",
            "input": "a<!--x<!--y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x<!--y)", "Text(data=b)", "EOF"],
            "diagnostics": ["nested-comment"],
        },
        {
            "id": "comment-end-dash-nonclose",
            "description": "a single dash before ordinary data is preserved",
            "input": "a<!--x-y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x-y)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-end-double-dash-nonclose",
            "description": "double dash followed by data remains part of the comment",
            "input": "a<!--x--y-->b",
            "tokens": ["Text(data=a)", "Comment(data=x--y)", "Text(data=b)", "EOF"],
        },
        {
            "id": "comment-end-dash-null",
            "description": "NULL after an end dash preserves the pending dash before replacement",
            "input": "a<!--x-\u0000-->b",
            "tokens": ["Text(data=a)", "Comment(data=x-�)", "Text(data=b)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
        },
        {
            "id": "comment-end-double-dash-null",
            "description": "NULL after double dash preserves both pending dashes before replacement",
            "input": "a<!--x--\u0000-->b",
            "tokens": ["Text(data=a)", "Comment(data=x--�)", "Text(data=b)", "EOF"],
            "diagnostics": ["unexpected-null-character"],
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
            "id": "comment-end-bang-dash-close",
            "description": "dash after --! re-enters end-dash handling and keeps greater-than literal",
            "input": "a<!--x--!->b",
            "tokens": ["Text(data=a)", "Comment(data=x--!->b)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
    ]


def comment_eof_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "comment-eof-body",
            "description": "EOF in comment body emits the current comment with eof-in-comment",
            "input": "a<!--open",
            "tokens": ["Text(data=a)", "Comment(data=open)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-start",
            "description": "EOF in comment start state emits an empty comment",
            "input": "a<!--",
            "tokens": ["Text(data=a)", "Comment(data=)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-start-dash",
            "description": "EOF after one initial dash emits an empty comment",
            "input": "a<!---",
            "tokens": ["Text(data=a)", "Comment(data=)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-less-than",
            "description": "EOF after a less-than sign preserves it in the comment",
            "input": "a<!--x<",
            "tokens": ["Text(data=a)", "Comment(data=x<)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-less-than-bang",
            "description": "EOF after less-than bang preserves both characters",
            "input": "a<!--x<!",
            "tokens": ["Text(data=a)", "Comment(data=x<!)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-less-than-bang-dash",
            "description": "EOF in less-than bang dash state emits without appending the pending dash",
            "input": "a<!--x<!-",
            "tokens": ["Text(data=a)", "Comment(data=x<!)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-less-than-bang-dash-dash",
            "description": "EOF after nested opener punctuation follows end-state EOF recovery",
            "input": "a<!--x<!--",
            "tokens": ["Text(data=a)", "Comment(data=x<!)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-end-dash",
            "description": "EOF after one end dash emits the current comment",
            "input": "a<!--x-",
            "tokens": ["Text(data=a)", "Comment(data=x)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-end",
            "description": "EOF after double dash emits the current comment",
            "input": "a<!--x--",
            "tokens": ["Text(data=a)", "Comment(data=x)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
        {
            "id": "comment-eof-end-bang",
            "description": "EOF after --! appends that pending sequence before emission",
            "input": "a<!--x--!",
            "tokens": ["Text(data=a)", "Comment(data=x--!)", "EOF"],
            "diagnostics": ["eof-in-comment"],
        },
    ]


def bogus_comment_cases() -> list[dict[str, object]]:
    return [
        {
            "id": "disallowed-processing-instruction-target-comment",
            "description": "the reserved xml processing-instruction target recovers as a bogus comment",
            "input": "a<?xml?>b",
            "tokens": ["Text(data=a)", "Comment(data=?xml?)", "Text(data=b)", "EOF"],
            "diagnostics": ["disallowed-processing-instruction-target"],
        },
        {
            "id": "processing-instruction-target-eof",
            "description": "EOF in a processing-instruction target discards the incomplete token",
            "input": "a<?xml",
            "tokens": ["Text(data=a)", "EOF"],
            "diagnostics": ["eof-in-processing-instruction"],
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
            "id": "bogus-comment-null",
            "description": "NULL inside bogus comments is replaced before emission",
            "input": "a<!foo\u0000bar>b",
            "tokens": ["Text(data=a)", "Comment(data=foo�bar)", "Text(data=b)", "EOF"],
            "diagnostics": ["incorrectly-opened-comment", "unexpected-null-character"],
        },
        {
            "id": "bogus-comment-eof",
            "description": "EOF in bogus comment emits the current comment without eof-in-comment",
            "input": "a<!foo",
            "tokens": ["Text(data=a)", "Comment(data=foo)", "EOF"],
            "diagnostics": ["incorrectly-opened-comment"],
        },
        {
            "id": "invalid-end-tag-open-bogus-comment",
            "description": "malformed end-tag openers recover as bogus comments",
            "input": "a</3>b",
            "tokens": ["Text(data=a)", "Comment(data=3)", "Text(data=b)", "EOF"],
            "diagnostics": ["invalid-first-character-of-tag-name"],
        },
    ]


def seeded_comment_cases() -> list[dict[str, object]]:
    return [
        seeded(
            "seeded-comment-start-abrupt-close",
            "comment start state closes abruptly on greater-than",
            "Comment start state",
            "",
            ">after",
            ["Comment(data=)", "Text(data=after)", "EOF"],
            ["abrupt-closing-of-empty-comment"],
        ),
        seeded(
            "seeded-comment-start-null",
            "comment start state replaces NULL before continuing",
            "Comment start state",
            "seed",
            "\u0000-->after",
            ["Comment(data=seed�)", "Text(data=after)", "EOF"],
            ["unexpected-null-character"],
        ),
        seeded(
            "seeded-comment-start-dash-abrupt-close",
            "comment start dash state closes abruptly on greater-than",
            "Comment start dash state",
            "",
            ">after",
            ["Comment(data=)", "Text(data=after)", "EOF"],
            ["abrupt-closing-of-empty-comment"],
        ),
        seeded(
            "seeded-comment-start-dash-null",
            "comment start dash state appends the pending dash before NULL replacement",
            "Comment start dash state",
            "seed",
            "\u0000-->after",
            ["Comment(data=seed-�)", "Text(data=after)", "EOF"],
            ["unexpected-null-character"],
        ),
        seeded(
            "seeded-comment-body-close",
            "comment body continuation appends text before the close",
            "Comment state",
            "seed",
            " tail-->after",
            ["Comment(data=seed tail)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-body-null",
            "comment body continuation replaces NULL before closing",
            "Comment state",
            "seed",
            "\u0000-->after",
            ["Comment(data=seed�)", "Text(data=after)", "EOF"],
            ["unexpected-null-character"],
        ),
        seeded(
            "seeded-comment-less-than-text",
            "less-than continuation preserves the already-appended less-than sign",
            "Comment less-than sign state",
            "seed<",
            "x-->after",
            ["Comment(data=seed<x)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-less-than-repeat",
            "less-than continuation can remain in the less-than state",
            "Comment less-than sign state",
            "seed<",
            "<x-->after",
            ["Comment(data=seed<<x)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-less-than-bang-text",
            "less-than bang continuation falls back to comment body on text",
            "Comment less-than sign bang state",
            "seed<!",
            "x-->after",
            ["Comment(data=seed<!x)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-less-than-bang-dash-text",
            "less-than bang dash continuation appends the pending dash via end-dash state",
            "Comment less-than sign bang dash state",
            "seed<!",
            "x-->after",
            ["Comment(data=seed<!-x)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-less-than-bang-dash-dash-nested",
            "less-than bang dash dash reports nested-comment before end-state recovery",
            "Comment less-than sign bang dash dash state",
            "seed<!",
            "x-->after",
            ["Comment(data=seed<!--x)", "Text(data=after)", "EOF"],
            ["nested-comment"],
        ),
        seeded(
            "seeded-comment-end-dash-text",
            "end dash continuation preserves the pending dash before text",
            "Comment end dash state",
            "seed",
            "x-->after",
            ["Comment(data=seed-x)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-end-dash-null",
            "end dash continuation preserves dash before NULL replacement",
            "Comment end dash state",
            "seed",
            "\u0000-->after",
            ["Comment(data=seed-�)", "Text(data=after)", "EOF"],
            ["unexpected-null-character"],
        ),
        seeded(
            "seeded-comment-end-close",
            "comment end continuation emits on greater-than",
            "Comment end state",
            "seed",
            ">after",
            ["Comment(data=seed)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-end-text",
            "comment end continuation appends pending double dash before text",
            "Comment end state",
            "seed",
            "x-->after",
            ["Comment(data=seed--x)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-end-null",
            "comment end continuation appends double dash before NULL replacement",
            "Comment end state",
            "seed",
            "\u0000-->after",
            ["Comment(data=seed--�)", "Text(data=after)", "EOF"],
            ["unexpected-null-character"],
        ),
        seeded(
            "seeded-comment-end-bang-close",
            "comment end bang continuation emits with incorrectly-closed-comment",
            "Comment end bang state",
            "seed",
            ">after",
            ["Comment(data=seed)", "Text(data=after)", "EOF"],
            ["incorrectly-closed-comment"],
        ),
        seeded(
            "seeded-comment-end-bang-text",
            "comment end bang continuation appends --! before text",
            "Comment end bang state",
            "seed",
            "x-->after",
            ["Comment(data=seed--!x)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-comment-end-bang-eof",
            "comment end bang continuation appends --! before EOF emission",
            "Comment end bang state",
            "seed",
            "",
            ["Comment(data=seed--!)", "EOF"],
            ["eof-in-comment"],
        ),
    ]


def seeded_bogus_comment_cases() -> list[dict[str, object]]:
    return [
        seeded(
            "seeded-bogus-comment-close",
            "bogus comment continuation emits on greater-than",
            "Bogus comment state",
            "seed",
            " tail>after",
            ["Comment(data=seed tail)", "Text(data=after)", "EOF"],
        ),
        seeded(
            "seeded-bogus-comment-null",
            "bogus comment continuation replaces NULL before emission",
            "Bogus comment state",
            "seed",
            "\u0000>after",
            ["Comment(data=seed�)", "Text(data=after)", "EOF"],
            ["unexpected-null-character"],
        ),
        seeded(
            "seeded-bogus-comment-eof",
            "bogus comment continuation emits on EOF without eof-in-comment",
            "Bogus comment state",
            "seed",
            " tail",
            ["Comment(data=seed tail)", "EOF"],
        ),
    ]


def seeded(
    case_id: str,
    description: str,
    state: str,
    current_comment: str,
    input_text: str,
    tokens: list[str],
    diagnostics: list[str] | None = None,
) -> dict[str, object]:
    case: dict[str, object] = {
        "id": case_id,
        "description": description,
        "initial_state": state,
        "current_comment": current_comment,
        "input": input_text,
        "tokens": tokens,
    }
    if diagnostics:
        case["diagnostics"] = diagnostics
    return case


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
