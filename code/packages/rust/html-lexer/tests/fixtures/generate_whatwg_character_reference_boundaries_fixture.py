#!/usr/bin/env python3

"""Generate Venture's WHATWG tokenizer character-reference boundary fixture."""

from __future__ import annotations

import argparse
from pathlib import Path

from generated_fixture_io import write_fixture_json


DEFAULT_OUTPUT = Path(__file__).with_name(
    "whatwg-character-reference-boundaries.json"
)


def main() -> int:
    args = parse_args()
    output = Path(args.output).expanduser().resolve()
    return write_fixture_json(output, build_fixture(), check=args.check)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate WHATWG tokenizer character-reference boundary fixture JSON."
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
        "format": "whatwg-html-tokenizer-character-reference-boundaries/v1",
        "description": (
            "Character-reference boundary recovery for named, numeric, "
            "attribute, RCDATA, and seeded continuation tokenizer states."
        ),
        "cases": [normalize_case(case) for case in build_cases()],
    }


def build_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = [
        case("data-named-semicolon", "semicolon named reference in data", "a&amp;b", ["Text(data=a&b)", "EOF"]),
        case("data-legacy-missing-semicolon-before-space", "legacy semicolonless name before whitespace in data", "a&copy b", ["Text(data=a© b)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        case("data-nonlegacy-missing-semicolon-literal", "non-legacy semicolonless name stays literal", "a&trade b", ["Text(data=a&trade b)", "EOF"]),
        case("data-longest-prefix-semicolon", "longest semicolon named reference wins", "a&notin;b", ["Text(data=a∉b)", "EOF"]),
        case("data-legacy-prefix-in-longer-name", "legacy prefix decodes inside longer data names", "a&notin b", ["Text(data=a¬in b)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        case("data-unknown-name", "unknown name stays literal in data", "a&nosuch;b", ["Text(data=a&nosuch;b)", "EOF"]),
        case("data-ambiguous-name-text", "legacy text reference decodes before alphanumeric text", "a&copycat b", ["Text(data=a©cat b)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        case("attribute-named-semicolon", "semicolon named reference in attributes", '<p title="a&amp;b">', ["StartTag(name=p, attributes=[title=a&b], self_closing=false)", "EOF"]),
        case("attribute-ambiguous-preserved", "attribute ambiguous ampersand preserves alphanumeric suffix", '<p title="a&copycat b">', ["StartTag(name=p, attributes=[title=a&copycat b], self_closing=false)", "EOF"]),
        case("attribute-legacy-before-space", "legacy semicolonless name before whitespace in attributes", '<p title="a&copy b">', ["StartTag(name=p, attributes=[title=a© b], self_closing=false)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        case("attribute-unknown-name", "unknown name stays literal in attributes", '<p title="a&nosuch;b">', ["StartTag(name=p, attributes=[title=a&nosuch;b], self_closing=false)", "EOF"]),
        case("rcdata-named-semicolon", "semicolon named reference in RCDATA", "a&amp;</title>", ["Text(data=a&)", "EndTag(name=title)", "EOF"], initial_state="RCDATA state", last_start_tag="title"),
        case("rcdata-ambiguous-name-text", "RCDATA follows text ambiguous ampersand recovery", "a&copycat</title>", ["Text(data=a©cat)", "EndTag(name=title)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"], initial_state="RCDATA state", last_start_tag="title"),
        case("rcdata-legacy-prefix-in-longer-name", "legacy prefix decodes inside longer RCDATA names", "a&notin </title>", ["Text(data=a¬in )", "EndTag(name=title)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"], initial_state="RCDATA state", last_start_tag="title"),
        case("data-numeric-decimal-semicolon", "decimal numeric reference with semicolon", "a&#65;b", ["Text(data=aAb)", "EOF"]),
        case("data-numeric-hex-semicolon", "hex numeric reference with semicolon", "a&#x41;b", ["Text(data=aAb)", "EOF"]),
        case("data-numeric-decimal-missing-semicolon", "decimal numeric reference missing semicolon", "a&#65 b", ["Text(data=aA b)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        case("data-numeric-hex-missing-semicolon", "hex numeric reference missing semicolon", "a&#x41z", ["Text(data=aAz)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        case("data-numeric-decimal-zero", "numeric null maps to replacement character", "a&#0;b", ["Text(data=a�b)", "EOF"], diagnostics=["null-character-reference"]),
        case("data-numeric-surrogate", "numeric surrogate maps to replacement character", "a&#xD800;b", ["Text(data=a�b)", "EOF"], diagnostics=["surrogate-character-reference"]),
        case("data-numeric-outside-range", "outside-Unicode numeric reference maps to replacement character", "a&#x110000;b", ["Text(data=a�b)", "EOF"], diagnostics=["character-reference-outside-unicode-range"]),
        case("data-numeric-windows-1252", "Windows-1252 control reference remaps", "a&#x80;b", ["Text(data=a€b)", "EOF"], diagnostics=["control-character-reference"]),
        case("data-numeric-digitless-decimal", "digitless decimal reference stays literal", "a&#;b", ["Text(data=a&#;b)", "EOF"], diagnostics=["absence-of-digits-in-numeric-character-reference"]),
        case("data-numeric-digitless-hex", "digitless hex reference stays literal", "a&#x;b", ["Text(data=a&#x;b)", "EOF"], diagnostics=["absence-of-digits-in-numeric-character-reference"]),
        seeded("seeded-character-reference-named-data", "character-reference state dispatches to names", "Character reference state", "Data state", "&", "amp;!", ["Text(data=&!)", "EOF"]),
        seeded("seeded-character-reference-fallback-data", "character-reference state flushes literal ampersand", "Character reference state", "Data state", "&", " nope", ["Text(data=& nope)", "EOF"]),
        seeded("seeded-character-reference-eof-data", "character-reference state flushes temporary buffer at EOF", "Character reference state", "Data state", "&", "", ["Text(data=&)", "EOF"]),
        seeded("seeded-character-reference-numeric-data", "character-reference state dispatches to numeric references", "Character reference state", "Data state", "&", "#65;!", ["Text(data=A!)", "EOF"]),
        seeded("seeded-character-reference-named-rcdata", "character-reference state returns to RCDATA", "Character reference state", "RCDATA state", "&", "amp;</title>", ["Text(data=&)", "EndTag(name=title)", "EOF"], last_start_tag="title"),
        seeded("seeded-named-reference-complete-data", "named-reference state completes an entity name", "Named character reference state", "Data state", "&co", "py;!", ["Text(data=©!)", "EOF"]),
        seeded("seeded-named-reference-ambiguous-data", "named-reference state handles legacy prefixes in data", "Named character reference state", "Data state", "&copy", "cat!", ["Text(data=©cat!)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        seeded("seeded-named-reference-nonlegacy-literal", "named-reference state preserves non-legacy semicolonless names", "Named character reference state", "Data state", "&trade", "!", ["Text(data=&trade!)", "EOF"]),
        seeded("seeded-named-reference-eof", "named-reference state flushes partial names at EOF", "Named character reference state", "Data state", "&co", "", ["Text(data=&co)", "EOF"]),
        seeded("seeded-named-reference-rcdata-end-tag", "named-reference state returns to RCDATA before an end tag", "Named character reference state", "RCDATA state", "&a", "mp;</title>", ["Text(data=&)", "EndTag(name=title)", "EOF"], last_start_tag="title"),
        seeded("seeded-numeric-reference-decimal", "numeric-reference state continues decimal digits", "Numeric character reference state", "Data state", "&#", "65;!", ["Text(data=A!)", "EOF"]),
        seeded("seeded-numeric-reference-hex", "numeric-reference state dispatches to hex digits", "Numeric character reference state", "Data state", "&#", "x41;!", ["Text(data=A!)", "EOF"]),
        seeded("seeded-numeric-reference-absent-digits", "numeric-reference state reports missing digits before boundary", "Numeric character reference state", "Data state", "&#", ";!", ["Text(data=&#;!)", "EOF"], diagnostics=["absence-of-digits-in-numeric-character-reference"]),
        seeded("seeded-numeric-reference-eof", "numeric-reference state reports missing digits at EOF", "Numeric character reference state", "Data state", "&#", "", ["Text(data=&#)", "EOF"], diagnostics=["absence-of-digits-in-numeric-character-reference"]),
        seeded("seeded-hex-start-reference-complete", "hex-start state continues hex digits", "Hexadecimal character reference start state", "Data state", "&#x", "41;!", ["Text(data=A!)", "EOF"]),
        seeded("seeded-hex-start-reference-absent-digits", "hex-start state reports missing digits before boundary", "Hexadecimal character reference start state", "Data state", "&#x", ";!", ["Text(data=&#x;!)", "EOF"], diagnostics=["absence-of-digits-in-numeric-character-reference"]),
        seeded("seeded-hex-start-reference-eof", "hex-start state reports missing digits at EOF", "Hexadecimal character reference start state", "Data state", "&#x", "", ["Text(data=&#x)", "EOF"], diagnostics=["absence-of-digits-in-numeric-character-reference"]),
        seeded("seeded-hex-reference-missing-semicolon", "hex state decodes and reports missing semicolon", "Hexadecimal character reference state", "Data state", "&#x4", "1!", ["Text(data=A!)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        seeded("seeded-hex-reference-null", "hex state decodes null as replacement character", "Hexadecimal character reference state", "Data state", "&#x0", ";!", ["Text(data=�!)", "EOF"], diagnostics=["null-character-reference"]),
        seeded("seeded-decimal-reference-complete", "decimal state completes decimal digits", "Decimal character reference state", "Data state", "&#6", "5;!", ["Text(data=A!)", "EOF"]),
        seeded("seeded-decimal-reference-missing-semicolon", "decimal state decodes and reports missing semicolon", "Decimal character reference state", "Data state", "&#6", "5!", ["Text(data=A!)", "EOF"], diagnostics=["missing-semicolon-after-character-reference"]),
        seeded("seeded-decimal-reference-outside-range", "decimal state decodes outside-Unicode values as replacement", "Decimal character reference state", "Data state", "&#1114112", ";!", ["Text(data=�!)", "EOF"], diagnostics=["character-reference-outside-unicode-range"]),
        seeded("seeded-decimal-reference-rcdata", "decimal state returns to RCDATA before an end tag", "Decimal character reference state", "RCDATA state", "&#6", "5;</title>", ["Text(data=A)", "EndTag(name=title)", "EOF"], last_start_tag="title"),
    ]
    return cases


def case(
    case_id: str,
    description: str,
    input_text: str,
    tokens: list[str],
    *,
    diagnostics: list[str] | None = None,
    initial_state: str | None = None,
    last_start_tag: str | None = None,
) -> dict[str, object]:
    item: dict[str, object] = {
        "id": case_id,
        "description": description,
        "input": input_text,
        "tokens": tokens,
    }
    if diagnostics is not None:
        item["diagnostics"] = diagnostics
    if initial_state is not None:
        item["initial_state"] = initial_state
    if last_start_tag is not None:
        item["last_start_tag"] = last_start_tag
    return item


def seeded(
    case_id: str,
    description: str,
    initial_state: str,
    return_state: str,
    temporary_buffer: str,
    input_text: str,
    tokens: list[str],
    *,
    diagnostics: list[str] | None = None,
    last_start_tag: str | None = None,
) -> dict[str, object]:
    item = case(
        case_id,
        description,
        input_text,
        tokens,
        diagnostics=diagnostics,
        initial_state=initial_state,
        last_start_tag=last_start_tag,
    )
    item["return_state"] = return_state
    item["temporary_buffer"] = temporary_buffer
    return item


def normalize_case(case: dict[str, object]) -> dict[str, object]:
    normalized = dict(case)
    normalized.setdefault("diagnostics", [])
    return normalized


if __name__ == "__main__":
    raise SystemExit(main())
