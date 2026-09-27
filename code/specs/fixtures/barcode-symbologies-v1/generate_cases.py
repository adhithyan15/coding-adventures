#!/usr/bin/env python3
"""Generate the closed barcode-symbologies v1 fixture corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections.abc import Callable
from pathlib import Path
from typing import Any

MAX_INPUT_SCALARS = 4096
MAX_CASES = 96
MAX_FIXTURE_BYTES = 131_072
MAX_FIXTURE_DEPTH = 8
MAX_MODULE_BITS = 65_567
MAX_RUNS = 40_979
ERROR_IDS = [
    "input-too-long",
    "invalid-length",
    "invalid-character",
    "reserved-character",
    "invalid-guard",
    "guard-conflict",
    "invalid-check-digit",
]

ITF_WIDTHS = [
    "NNWWN",
    "WNNNW",
    "NWNNW",
    "WWNNN",
    "NNWNW",
    "WNWNN",
    "NWWNN",
    "NNNWW",
    "WNNWN",
    "NWNWN",
]
CODE39_BAR_SPACE = {
    "0": "bwbWBwBwb",
    "1": "BwbWbwbwB",
    "2": "bwBWbwbwB",
    "3": "BwBWbwbwb",
    "4": "bwbWBwbwB",
    "5": "BwbWBwbwb",
    "6": "bwBWBwbwb",
    "7": "bwbWbwBwB",
    "8": "BwbWbwBwb",
    "9": "bwBWbwBwb",
    "A": "BwbwbWbwB",
    "B": "bwBwbWbwB",
    "C": "BwBwbWbwb",
    "D": "bwbwBWbwB",
    "E": "BwbwBWbwb",
    "F": "bwBwBWbwb",
    "G": "bwbwbWBwB",
    "H": "BwbwbWBwb",
    "I": "bwBwbWBwb",
    "J": "bwbwBWBwb",
    "K": "BwbwbwbWB",
    "L": "bwBwbwbWB",
    "M": "BwBwbwbWb",
    "N": "bwbwBwbWB",
    "O": "BwbwBwbWb",
    "P": "bwBwBwbWb",
    "Q": "bwbwbwBWB",
    "R": "BwbwbwBWb",
    "S": "bwBwbwBWb",
    "T": "bwbwBwBWb",
    "U": "BWbwbwbwB",
    "V": "bWBwbwbwB",
    "W": "BWBwbwbwb",
    "X": "bWbwBwbwB",
    "Y": "BWbwBwbwb",
    "Z": "bWBwBwbwb",
    "-": "bWbwbwBwB",
    ".": "BWbwbwBwb",
    " ": "bWBwbwBwb",
    "$": "bWbWbWbwb",
    "/": "bWbWbwbWb",
    "+": "bWbwbWbWb",
    "%": "bwbWbWbWb",
    "*": "bWbwBwBwb",
}
CODE39_WIDTHS = {
    char: "".join("W" if element.isupper() else "N" for element in pattern)
    for char, pattern in CODE39_BAR_SPACE.items()
}
CODABAR_PATTERNS = {
    "0": "101010011",
    "1": "101011001",
    "2": "101001011",
    "3": "110010101",
    "4": "101101001",
    "5": "110101001",
    "6": "100101011",
    "7": "100101101",
    "8": "100110101",
    "9": "110100101",
    "-": "101001101",
    "$": "101100101",
    ":": "1101011011",
    "/": "1101101011",
    ".": "1101101101",
    "+": "1011011011",
    "A": "1011001001",
    "B": "1001001011",
    "C": "1010010011",
    "D": "1010011001",
}
CODE128_PATTERNS = [
    "11011001100",
    "11001101100",
    "11001100110",
    "10010011000",
    "10010001100",
    "10001001100",
    "10011001000",
    "10011000100",
    "10001100100",
    "11001001000",
    "11001000100",
    "11000100100",
    "10110011100",
    "10011011100",
    "10011001110",
    "10111001100",
    "10011101100",
    "10011100110",
    "11001110010",
    "11001011100",
    "11001001110",
    "11011100100",
    "11001110100",
    "11101101110",
    "11101001100",
    "11100101100",
    "11100100110",
    "11101100100",
    "11100110100",
    "11100110010",
    "11011011000",
    "11011000110",
    "11000110110",
    "10100011000",
    "10001011000",
    "10001000110",
    "10110001000",
    "10001101000",
    "10001100010",
    "11010001000",
    "11000101000",
    "11000100010",
    "10110111000",
    "10110001110",
    "10001101110",
    "10111011000",
    "10111000110",
    "10001110110",
    "11101110110",
    "11010001110",
    "11000101110",
    "11011101000",
    "11011100010",
    "11011101110",
    "11101011000",
    "11101000110",
    "11100010110",
    "11101101000",
    "11101100010",
    "11100011010",
    "11101111010",
    "11001000010",
    "11110001010",
    "10100110000",
    "10100001100",
    "10010110000",
    "10010000110",
    "10000101100",
    "10000100110",
    "10110010000",
    "10110000100",
    "10011010000",
    "10011000010",
    "10000110100",
    "10000110010",
    "11000010010",
    "11001010000",
    "11110111010",
    "11000010100",
    "10001111010",
    "10100111100",
    "10010111100",
    "10010011110",
    "10111100100",
    "10011110100",
    "10011110010",
    "11110100100",
    "11110010100",
    "11110010010",
    "11011011110",
    "11011110110",
    "11110110110",
    "10101111000",
    "10100011110",
    "10001011110",
    "10111101000",
    "10111100010",
    "11110101000",
    "11110100010",
    "10111011110",
    "10111101110",
    "11101011110",
    "11110101110",
    "11010000100",
    "11010010000",
    "11010011100",
    "1100011101011",
]
L_PATTERNS = [
    "0001101",
    "0011001",
    "0010011",
    "0111101",
    "0100011",
    "0110001",
    "0101111",
    "0111011",
    "0110111",
    "0001011",
]
G_PATTERNS = [
    "0100111",
    "0110011",
    "0011011",
    "0100001",
    "0011101",
    "0111001",
    "0000101",
    "0010001",
    "0001001",
    "0010111",
]
R_PATTERNS = [
    "1110010",
    "1100110",
    "1101100",
    "1000010",
    "1011100",
    "1001110",
    "1010000",
    "1000100",
    "1001000",
    "1110100",
]
EAN_PARITY = [
    "LLLLLL",
    "LLGLGG",
    "LLGGLG",
    "LLGGGL",
    "LGLLGG",
    "LGGLLG",
    "LGGGLL",
    "LGLGLG",
    "LGLGGL",
    "LGGLGL",
]


class ContractError(Exception):
    def __init__(self, error_id: str) -> None:
        self.error_id = error_id


def fail(error_id: str) -> None:
    raise ContractError(error_id)


def check_limit(text: str) -> None:
    if len(text) > MAX_INPUT_SCALARS:
        fail("input-too-long")


def ascii_upper(text: str) -> str:
    return "".join(chr(ord(char) - 32) if "a" <= char <= "z" else char for char in text)


def run_lengths(modules: str) -> list[int]:
    lengths: list[int] = []
    previous = modules[0]
    count = 0
    for bit in modules:
        if bit == previous:
            count += 1
        else:
            lengths.append(count)
            previous = bit
            count = 1
    lengths.append(count)
    return lengths


def exact(normalized: str, modules: str, **extra: Any) -> dict[str, Any]:
    return {
        "normalized": normalized,
        "modules": modules,
        "module_count": len(modules),
        "module_sha256": hashlib.sha256(modules.encode("ascii")).hexdigest(),
        "run_lengths": run_lengths(modules),
        **extra,
    }


def digest(normalized: str, modules: str) -> dict[str, Any]:
    lengths = run_lengths(modules)
    encoded_lengths = json.dumps(lengths, separators=(",", ":")).encode("ascii")
    return {
        "normalized_sha256": hashlib.sha256(normalized.encode("ascii")).hexdigest(),
        "module_count": len(modules),
        "module_sha256": hashlib.sha256(modules.encode("ascii")).hexdigest(),
        "run_count": len(lengths),
        "run_lengths_sha256": hashlib.sha256(encoded_lengths).hexdigest(),
    }


def encode_itf(text: str) -> dict[str, Any]:
    check_limit(text)
    if not text or len(text) % 2:
        fail("invalid-length")
    if any(char < "0" or char > "9" for char in text):
        fail("invalid-character")
    modules = "1010"
    for offset in range(0, len(text), 2):
        bars = ITF_WIDTHS[int(text[offset])]
        spaces = ITF_WIDTHS[int(text[offset + 1])]
        for bar, space in zip(bars, spaces, strict=True):
            modules += "1" * (3 if bar == "W" else 1)
            modules += "0" * (3 if space == "W" else 1)
    return exact(text, modules + "11101")


def encode_code39(text: str) -> dict[str, Any]:
    check_limit(text)
    normalized = ascii_upper(text)
    for char in normalized:
        if char == "*":
            fail("reserved-character")
        if char not in CODE39_WIDTHS:
            fail("invalid-character")
    parts: list[str] = []
    for char in f"*{normalized}*":
        pattern = CODE39_WIDTHS[char]
        parts.append(
            "".join(
                ("1" if index % 2 == 0 else "0") * (3 if width == "W" else 1)
                for index, width in enumerate(pattern)
            )
        )
    return exact(normalized, "0".join(parts))


def encode_codabar(text: str, options: dict[str, str]) -> dict[str, Any]:
    check_limit(text)
    normalized = ascii_upper(text)
    start_option = ascii_upper(options.get("start", "A"))
    stop_option = ascii_upper(options.get("stop", "A"))
    if start_option not in "ABCD" or stop_option not in "ABCD":
        fail("invalid-guard")
    if len(normalized) == 1 and normalized in "ABCD":
        fail("invalid-guard")
    starts_guard = len(normalized) >= 2 and normalized[0] in "ABCD"
    ends_guard = len(normalized) >= 2 and normalized[-1] in "ABCD"
    if starts_guard != ends_guard:
        fail("invalid-guard")
    if starts_guard and ends_guard:
        if ("start" in options and start_option != normalized[0]) or (
            "stop" in options and stop_option != normalized[-1]
        ):
            fail("guard-conflict")
        full = normalized
        body = normalized[1:-1]
    else:
        full = start_option + normalized + stop_option
        body = normalized
    if any(char in "ABCD" for char in body):
        fail("invalid-guard")
    if any(char not in CODABAR_PATTERNS for char in body):
        fail("invalid-character")
    return exact(full, "0".join(CODABAR_PATTERNS[char] for char in full))


def encode_code128(text: str) -> dict[str, Any]:
    check_limit(text)
    if any(ord(char) < 32 or ord(char) > 126 for char in text):
        fail("invalid-character")
    data_values = [ord(char) - 32 for char in text]
    checksum = (
        104 + sum(value * (index + 1) for index, value in enumerate(data_values))
    ) % 103
    values = [104, *data_values, checksum, 106]
    return exact(
        text,
        "".join(CODE128_PATTERNS[value] for value in values),
        checksum=checksum,
        symbol_values=values,
    )


def digits_with_length(text: str, lengths: tuple[int, ...]) -> None:
    check_limit(text)
    if len(text) not in lengths:
        fail("invalid-length")
    if any(char < "0" or char > "9" for char in text):
        fail("invalid-character")


def retail_check(payload: str) -> str:
    total = sum(
        int(char) * (3 if index % 2 == 0 else 1)
        for index, char in enumerate(reversed(payload))
    )
    return str((10 - total % 10) % 10)


def encode_upc(text: str) -> dict[str, Any]:
    digits_with_length(text, (11, 12))
    expected = retail_check(text[:11])
    if len(text) == 12 and text[-1] != expected:
        fail("invalid-check-digit")
    normalized = text if len(text) == 12 else text + expected
    modules = (
        "101"
        + "".join(L_PATTERNS[int(char)] for char in normalized[:6])
        + "01010"
        + "".join(R_PATTERNS[int(char)] for char in normalized[6:])
        + "101"
    )
    return exact(normalized, modules, checksum=expected)


def encode_ean(text: str) -> dict[str, Any]:
    digits_with_length(text, (12, 13))
    expected = retail_check(text[:12])
    if len(text) == 13 and text[-1] != expected:
        fail("invalid-check-digit")
    normalized = text if len(text) == 13 else text + expected
    parity = EAN_PARITY[int(normalized[0])]
    left = "".join(
        (L_PATTERNS if encoding == "L" else G_PATTERNS)[int(char)]
        for char, encoding in zip(normalized[1:7], parity, strict=True)
    )
    right = "".join(R_PATTERNS[int(char)] for char in normalized[7:])
    return exact(
        normalized,
        "101" + left + "01010" + right + "101",
        checksum=expected,
        left_parity=parity,
    )


ENCODERS: dict[str, Callable[..., dict[str, Any]]] = {
    "itf": encode_itf,
    "code39": encode_code39,
    "codabar": encode_codabar,
    "code128-b": encode_code128,
    "upc-a": encode_upc,
    "ean-13": encode_ean,
}


def materialize(descriptor: dict[str, Any]) -> str:
    if "text" in descriptor:
        return descriptor["text"]
    repeat = descriptor["repeat"]
    return repeat["text"] * repeat["count"]


def definition(
    case_id: str,
    symbology: str,
    text: str | None = None,
    *,
    repeat: tuple[str, int] | None = None,
    options: dict[str, str] | None = None,
    compact: bool = False,
) -> dict[str, Any]:
    input_descriptor = (
        {"text": text}
        if repeat is None
        else {"repeat": {"text": repeat[0], "count": repeat[1]}}
    )
    return {
        "id": case_id,
        "symbology": symbology,
        "input": input_descriptor,
        "options": options or {},
        "compact": compact,
    }


def definitions() -> list[dict[str, Any]]:
    cases = [
        definition("barcode-v1-itf-12", "itf", "12"),
        definition("barcode-v1-itf-123456", "itf", "123456"),
        definition("barcode-v1-itf-all-digits", "itf", "00112233445566778899"),
        definition("barcode-v1-itf-limit", "itf", repeat=("0", 4096), compact=True),
        definition("barcode-v1-itf-empty", "itf", ""),
        definition("barcode-v1-itf-odd", "itf", "1"),
        definition("barcode-v1-itf-length-before-character", "itf", "A"),
        definition("barcode-v1-itf-invalid-character", "itf", "12A4"),
        definition("barcode-v1-itf-unicode-digit", "itf", "１２"),
        definition("barcode-v1-itf-over-limit", "itf", repeat=("0", 4097)),
        definition("barcode-v1-code39-empty", "code39", ""),
        definition("barcode-v1-code39-a", "code39", "A"),
        definition("barcode-v1-code39-ascii-fold", "code39", "abc"),
        definition(
            "barcode-v1-code39-full-alphabet",
            "code39",
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%",
        ),
        definition(
            "barcode-v1-code39-limit", "code39", repeat=("A", 4096), compact=True
        ),
        definition("barcode-v1-code39-reserved-star", "code39", "*"),
        definition("barcode-v1-code39-invalid-before-star", "code39", "@*"),
        definition("barcode-v1-code39-star-before-invalid", "code39", "*@"),
        definition("barcode-v1-code39-invalid-character", "code39", "@"),
        definition("barcode-v1-code39-unicode-fold", "code39", "ß"),
        definition("barcode-v1-code39-over-limit", "code39", repeat=("A", 4097)),
        definition("barcode-v1-codabar-empty", "codabar", ""),
        definition("barcode-v1-codabar-zero", "codabar", "0"),
        definition("barcode-v1-codabar-full", "codabar", "A123B"),
        definition(
            "barcode-v1-codabar-custom-guards",
            "codabar",
            "0",
            options={"start": "B", "stop": "D"},
        ),
        definition(
            "barcode-v1-codabar-one-sided-option",
            "codabar",
            "0",
            options={"start": "B"},
        ),
        definition(
            "barcode-v1-codabar-lowercase-options",
            "codabar",
            "0",
            options={"start": "b", "stop": "d"},
        ),
        definition(
            "barcode-v1-codabar-agreeing-guard-options",
            "codabar",
            "A12B",
            options={"start": "a", "stop": "b"},
        ),
        definition("barcode-v1-codabar-ascii-fold", "codabar", "b40156d"),
        definition(
            "barcode-v1-codabar-full-alphabet",
            "codabar",
            "0123456789-$:/.+",
            options={"start": "C", "stop": "D"},
        ),
        definition(
            "barcode-v1-codabar-limit", "codabar", repeat=("0", 4096), compact=True
        ),
        definition("barcode-v1-codabar-partial-guard", "codabar", "A40156"),
        definition("barcode-v1-codabar-single-guard", "codabar", "A"),
        definition("barcode-v1-codabar-interior-guard", "codabar", "40A56"),
        definition(
            "barcode-v1-codabar-invalid-option", "codabar", "0", options={"start": "X"}
        ),
        definition(
            "barcode-v1-codabar-guard-conflict",
            "codabar",
            "A12B",
            options={"start": "C"},
        ),
        definition(
            "barcode-v1-codabar-conflict-before-character",
            "codabar",
            "A*B",
            options={"start": "C"},
        ),
        definition("barcode-v1-codabar-invalid-character", "codabar", "12*34"),
        definition("barcode-v1-codabar-unicode-digit", "codabar", "１２"),
        definition("barcode-v1-codabar-over-limit", "codabar", repeat=("0", 4097)),
        definition("barcode-v1-code128-b-empty", "code128-b", ""),
        definition("barcode-v1-code128-b-a", "code128-b", "A"),
        definition("barcode-v1-code128-b-text", "code128-b", "Code 128"),
        definition(
            "barcode-v1-code128-b-printable-ascii",
            "code128-b",
            "".join(chr(value) for value in range(32, 127)),
        ),
        definition("barcode-v1-code128-b-checksum-95", "code128-b", "~"),
        definition("barcode-v1-code128-b-checksum-96", "code128-b", "!O"),
        definition("barcode-v1-code128-b-checksum-97", "code128-b", " P"),
        definition("barcode-v1-code128-b-checksum-98", "code128-b", "!P"),
        definition("barcode-v1-code128-b-checksum-99", "code128-b", " Q"),
        definition("barcode-v1-code128-b-checksum-100", "code128-b", "!Q"),
        definition("barcode-v1-code128-b-checksum-101", "code128-b", " R"),
        definition("barcode-v1-code128-b-checksum-102", "code128-b", "!R"),
        definition(
            "barcode-v1-code128-b-limit", "code128-b", repeat=("A", 4096), compact=True
        ),
        definition("barcode-v1-code128-b-control", "code128-b", "\u0000"),
        definition("barcode-v1-code128-b-del", "code128-b", "\u007f"),
        definition("barcode-v1-code128-b-unicode", "code128-b", "é"),
        definition(
            "barcode-v1-code128-b-astral-at-limit", "code128-b", repeat=("😀", 4096)
        ),
        definition(
            "barcode-v1-code128-b-astral-over-limit", "code128-b", repeat=("😀", 4097)
        ),
        definition("barcode-v1-code128-b-over-limit", "code128-b", repeat=("A", 4097)),
        definition("barcode-v1-upc-a-computed", "upc-a", "03600029145"),
        definition("barcode-v1-upc-a-supplied", "upc-a", "036000291452"),
        definition("barcode-v1-upc-a-leading-zero", "upc-a", "00000000000"),
        definition("barcode-v1-upc-a-table-coverage", "upc-a", "12457836780"),
        definition("barcode-v1-upc-a-left-nine", "upc-a", "90000000000"),
        definition("barcode-v1-upc-a-empty", "upc-a", ""),
        definition("barcode-v1-upc-a-invalid-length", "upc-a", "123"),
        definition("barcode-v1-upc-a-length-before-character", "upc-a", "A"),
        definition("barcode-v1-upc-a-invalid-character", "upc-a", "0360002914A"),
        definition("barcode-v1-upc-a-unicode-digit", "upc-a", "０3600029145"),
        definition("barcode-v1-upc-a-check-mismatch", "upc-a", "036000291453"),
        definition("barcode-v1-upc-a-over-limit", "upc-a", repeat=("A", 4097)),
        definition("barcode-v1-ean-13-computed", "ean-13", "400638133393"),
        definition("barcode-v1-ean-13-supplied", "ean-13", "4006381333931"),
        definition("barcode-v1-ean-13-table-coverage", "ean-13", "677908900000"),
        definition("barcode-v1-ean-13-empty", "ean-13", ""),
        definition("barcode-v1-ean-13-invalid-length", "ean-13", "123"),
        definition("barcode-v1-ean-13-length-before-character", "ean-13", "A"),
        definition("barcode-v1-ean-13-invalid-character", "ean-13", "40063813339A"),
        definition("barcode-v1-ean-13-unicode-digit", "ean-13", "４00638133393"),
        definition("barcode-v1-ean-13-check-mismatch", "ean-13", "4006381333932"),
        definition("barcode-v1-ean-13-over-limit", "ean-13", repeat=("A", 4097)),
    ]
    for leading in range(10):
        cases.append(
            definition(
                f"barcode-v1-ean-13-parity-{leading}", "ean-13", f"{leading}12345678901"
            )
        )
    return cases


def build_document() -> dict[str, Any]:
    cases: list[dict[str, Any]] = []
    for item in definitions():
        text = materialize(item["input"])
        try:
            encoder = ENCODERS[item["symbology"]]
            result = (
                encoder(text, item["options"])
                if item["symbology"] == "codabar"
                else encoder(text)
            )
            expected = (
                digest(result["normalized"], result["modules"])
                if item["compact"]
                else result
            )
        except ContractError as error:
            expected = {"error": error.error_id}
        cases.append(
            {key: item[key] for key in ("id", "symbology", "input", "options")}
            | {"expected": expected}
        )
    return {
        "schema_version": 1,
        "profile": "barcode-symbologies-v1",
        "limits": {
            "max_input_scalars": MAX_INPUT_SCALARS,
            "max_cases": MAX_CASES,
            "max_fixture_bytes": MAX_FIXTURE_BYTES,
            "max_fixture_depth": MAX_FIXTURE_DEPTH,
            "max_module_bits": MAX_MODULE_BITS,
            "max_runs": MAX_RUNS,
        },
        "error_ids": ERROR_IDS,
        "cases": cases,
    }


def render_document() -> str:
    return json.dumps(build_document(), indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    destination = Path(__file__).with_name("cases.json")
    rendered = render_document()
    encoded = rendered.encode("utf-8")
    if args.check:
        if not destination.exists() or destination.read_bytes() != encoded:
            print("barcode-symbologies-v1/cases.json is stale")
            return 1
        return 0
    destination.write_bytes(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
