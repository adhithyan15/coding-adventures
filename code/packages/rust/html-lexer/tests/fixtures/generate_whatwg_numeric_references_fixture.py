#!/usr/bin/env python3

"""Generate Venture's WHATWG numeric character reference edge fixture.

The numeric character reference algorithm is finite around its interesting
error classes: null, controls, Windows-1252 remaps, surrogates, noncharacters,
Unicode boundaries, and outside-range values. This fixture encodes those edge
classes directly so Rust tests can sweep decimal/hexadecimal and
semicolon/missing-semicolon forms without duplicating the expectation table by
hand.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_OUTPUT = Path(__file__).with_name("whatwg-numeric-references.json")

WINDOWS_1252_REPLACEMENTS = {
    0x80: 0x20AC,
    0x82: 0x201A,
    0x83: 0x0192,
    0x84: 0x201E,
    0x85: 0x2026,
    0x86: 0x2020,
    0x87: 0x2021,
    0x88: 0x02C6,
    0x89: 0x2030,
    0x8A: 0x0160,
    0x8B: 0x2039,
    0x8C: 0x0152,
    0x8E: 0x017D,
    0x91: 0x2018,
    0x92: 0x2019,
    0x93: 0x201C,
    0x94: 0x201D,
    0x95: 0x2022,
    0x96: 0x2013,
    0x97: 0x2014,
    0x98: 0x02DC,
    0x99: 0x2122,
    0x9A: 0x0161,
    0x9B: 0x203A,
    0x9C: 0x0153,
    0x9E: 0x017E,
    0x9F: 0x0178,
}


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
        description="Generate WHATWG numeric character reference edge fixture JSON."
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
    cases = [case_for_value(value) for value in numeric_values()]
    return {
        "format": "whatwg-html-numeric-character-references/v1",
        "description": (
            "Numeric character reference edge classes from the WHATWG HTML "
            "tokenization algorithm."
        ),
        "cases": cases,
    }


def numeric_values() -> list[int]:
    values: set[int] = set()

    values.update(range(0x00, 0x20))
    values.update(range(0x7F, 0xA0))
    values.update(range(0xD800, 0xE000))
    values.update(range(0xFDD0, 0xFDF0))

    for plane in range(0x00, 0x11):
        base = plane << 16
        values.add(base + 0xFFFE)
        values.add(base + 0xFFFF)
        if base + 0xFFFD <= 0x10FFFF:
            values.add(base + 0xFFFD)

    values.update(
        {
            0x20,
            0x41,
            0x7E,
            0xA0,
            0xD7FE,
            0xD7FF,
            0xE000,
            0xE001,
            0xFDCF,
            0xFDF0,
            0x10000,
            0x1F600,
            0x10FFFD,
            0x10FFFE,
            0x10FFFF,
            0x110000,
            0xFFFFFFFF,
        }
    )

    return sorted(values)


def case_for_value(value: int) -> dict[str, object]:
    codepoints, diagnostics = decode_numeric_reference(value)
    return {
        "value": value,
        "decimal": f"&#{value};",
        "hex": f"&#x{value:X};",
        "decimal_missing_semicolon": f"&#{value}",
        "hex_missing_semicolon": f"&#X{value:X}",
        "characters": "".join(chr(codepoint) for codepoint in codepoints),
        "codepoints": codepoints,
        "diagnostics": diagnostics,
    }


def decode_numeric_reference(value: int) -> tuple[list[int], list[str]]:
    if value == 0:
        return [0xFFFD], ["null-character-reference"]

    if value > 0x10FFFF:
        return [0xFFFD], ["character-reference-outside-unicode-range"]

    if 0xD800 <= value <= 0xDFFF:
        return [0xFFFD], ["surrogate-character-reference"]

    diagnostics: list[str] = []
    if is_noncharacter(value):
        diagnostics.append("noncharacter-character-reference")

    replacement = WINDOWS_1252_REPLACEMENTS.get(value)
    if replacement is not None:
        diagnostics.append("control-character-reference")
        return [replacement], diagnostics

    if is_control_character_reference(value):
        diagnostics.append("control-character-reference")

    return [value], diagnostics


def is_noncharacter(value: int) -> bool:
    return (0xFDD0 <= value <= 0xFDEF) or any(
        value == (plane << 16) + suffix
        for plane in range(0x00, 0x11)
        for suffix in (0xFFFE, 0xFFFF)
    )


def is_control_character_reference(value: int) -> bool:
    return value == 0x0D or (is_control(value) and not is_ascii_whitespace_control(value))


def is_control(value: int) -> bool:
    return value <= 0x1F or 0x7F <= value <= 0x9F


def is_ascii_whitespace_control(value: int) -> bool:
    return value in {0x09, 0x0A, 0x0C, 0x20}


if __name__ == "__main__":
    raise SystemExit(main())
