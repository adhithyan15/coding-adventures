"""Validate the language-neutral ZIP raw RFC 1951 v1 fixture contract."""

from __future__ import annotations

import binascii
import json
import re
import unittest
import zlib
from pathlib import Path
from typing import Any, ClassVar

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/zip-raw-rfc1951-v1"
ERROR_ID_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
HARD_MAX_OUTPUT = 256 * 1024 * 1024

EXPECTED_ERROR_IDS = {
    "invalid-output-limit",
    "unexpected-eof",
    "reserved-block-type",
    "stored-length-mismatch",
    "huffman-oversubscribed",
    "incomplete-code-length-tree",
    "incomplete-literal-length-tree",
    "incomplete-distance-tree",
    "repeat-without-previous",
    "repeat-overrun",
    "invalid-literal-length-symbol",
    "reserved-distance-symbol",
    "invalid-back-reference",
    "output-limit-exceeded",
}


def materialize_output(value: dict[str, object]) -> bytes:
    """Expand one closed fixture output descriptor."""

    if "hex" in value:
        encoded = value["hex"]
        if not isinstance(encoded, str):
            raise TypeError("output hex must be a string")
        return bytes.fromhex(encoded)
    repeated = value.get("repeat_hex")
    count = value.get("count")
    if not isinstance(repeated, str) or not isinstance(count, int):
        raise TypeError("repeated output must provide repeat_hex and count")
    return bytes.fromhex(repeated) * count


def raw_deflate(data: bytes) -> bytes:
    """Encode with the independent Python standard-library oracle."""

    compressor = zlib.compressobj(level=6, wbits=-15)
    return compressor.compress(data) + compressor.flush()


class _BitReader:
    def __init__(self, data: bytes) -> None:
        self.data = data
        self.position = 0

    def read_lsb(self, count: int) -> int:
        value = 0
        for bit_index in range(count):
            byte = self.data[self.position // 8]
            value |= ((byte >> (self.position % 8)) & 1) << bit_index
            self.position += 1
        return value

    def decode(self, lengths: list[int]) -> int:
        counts = [0] * 16
        for length in lengths:
            if length:
                counts[length] += 1
        next_code = [0] * 16
        code = 0
        for length in range(1, 16):
            code = (code + counts[length - 1]) << 1
            next_code[length] = code
        table: dict[tuple[int, int], int] = {}
        for symbol, length in enumerate(lengths):
            if length:
                table[(length, next_code[length])] = symbol
                next_code[length] += 1
        code = 0
        for length in range(1, 16):
            code = (code << 1) | self.read_lsb(1)
            if (length, code) in table:
                return table[(length, code)]
        raise AssertionError("fixture contains no decodable Huffman symbol")


def decode_dynamic_prefix(data: bytes) -> tuple[list[int], list[int], int, int]:
    """Return dynamic alphabets and the first length/distance symbols."""

    reader = _BitReader(data)
    if reader.read_lsb(1) != 1 or reader.read_lsb(2) != 2:
        raise AssertionError("fixture is not one final dynamic block")
    literal_count = reader.read_lsb(5) + 257
    distance_count = reader.read_lsb(5) + 1
    code_length_count = reader.read_lsb(4) + 4
    order = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]
    code_lengths = [0] * 19
    for index in range(code_length_count):
        code_lengths[order[index]] = reader.read_lsb(3)
    lengths: list[int] = []
    while len(lengths) < literal_count + distance_count:
        symbol = reader.decode(code_lengths)
        if symbol > 15:
            raise AssertionError("reserved-distance fixtures must use explicit lengths")
        lengths.append(symbol)
    literal_lengths = lengths[:literal_count]
    distance_lengths = lengths[literal_count:]
    return (
        literal_lengths,
        distance_lengths,
        reader.decode(literal_lengths),
        reader.decode(distance_lengths),
    )


class ZipRawRfc1951FixtureTests(unittest.TestCase):
    schema: ClassVar[dict[str, Any]]
    document: ClassVar[dict[str, Any]]
    validator: ClassVar[Any]

    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads((FIXTURE_ROOT / "schema.json").read_text("utf-8"))
        cls.document = json.loads((FIXTURE_ROOT / "cases.json").read_text("utf-8"))
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)

    def test_schema_profile_limits_ids_and_operations_are_closed(self) -> None:
        self.validator.validate(self.document)
        self.assertEqual(
            self.document["limits"],
            {
                "default_max_output": HARD_MAX_OUTPUT,
                "hard_max_output": HARD_MAX_OUTPUT,
            },
        )
        ids = [case["id"] for case in self.document["cases"]]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(
            {case["operation"] for case in self.document["cases"]},
            {"inflate", "inflate-error", "deflate-interoperability", "crc32"},
        )

    def test_error_ids_are_stable_payload_blind_and_exercised(self) -> None:
        error_ids = set(self.document["error_ids"])
        self.assertEqual(error_ids, EXPECTED_ERROR_IDS)
        exercised = {
            case["expected"]["error_id"]
            for case in self.document["cases"]
            if case["operation"] == "inflate-error"
        }
        self.assertEqual(exercised, EXPECTED_ERROR_IDS)
        for error_id in error_ids:
            self.assertRegex(error_id, ERROR_ID_RE)
            self.assertNotRegex(error_id, r"(?:0x|[0-9]{2,})")

    def test_output_limit_cases_pin_exact_boundary_semantics(self) -> None:
        for case in self.document["cases"]:
            operation = case["operation"]
            maximum = case.get("max_output")
            if operation == "inflate" and maximum is not None:
                self.assertGreaterEqual(maximum, 0, case["id"])
                self.assertLessEqual(maximum, HARD_MAX_OUTPUT, case["id"])
                output = materialize_output(case["expected"]["output"])
                self.assertLessEqual(len(output), maximum, case["id"])
            if operation != "inflate-error":
                continue
            error_id = case["expected"]["error_id"]
            if error_id == "invalid-output-limit":
                self.assertTrue(
                    maximum is not None and (maximum < 0 or maximum > HARD_MAX_OUTPUT),
                    case["id"],
                )
            elif error_id == "output-limit-exceeded":
                self.assertIsInstance(maximum, int, case["id"])
                decoder = zlib.decompressobj(wbits=-15)
                output = decoder.decompress(bytes.fromhex(case["input_hex"]))
                self.assertGreater(len(output), maximum, case["id"])

    def test_python_zlib_independently_decodes_foreign_inflate_cases(self) -> None:
        for case in self.document["cases"]:
            if case["operation"] != "inflate" or case.get("oracle") != "python-zlib":
                continue
            compressed = bytes.fromhex(case["input_hex"])
            decoder = zlib.decompressobj(wbits=-15)
            actual = decoder.decompress(compressed) + decoder.flush()
            expected = materialize_output(case["expected"]["output"])
            consumed = len(compressed) - len(decoder.unused_data)
            self.assertEqual(actual, expected, case["id"])
            self.assertEqual(consumed, case["expected"]["bytes_consumed"], case["id"])

    def test_hdist_32_vector_is_the_single_documented_zlib_exception(self) -> None:
        cases = [
            case
            for case in self.document["cases"]
            if case.get("oracle") == "rfc1951-hdist-32-zero-slots"
        ]
        self.assertEqual(len(cases), 1)
        case = cases[0]
        compressed = bytes.fromhex(case["input_hex"])
        first_bits = int.from_bytes(compressed[:2], "little")
        self.assertEqual(first_bits & 1, 1)  # BFINAL
        self.assertEqual((first_bits >> 1) & 0b11, 0b10)  # dynamic block
        self.assertEqual((first_bits >> 3) & 0b11111, 0)  # 257 LL slots
        self.assertEqual((first_bits >> 8) & 0b11111, 31)  # 32 distance slots
        self.assertEqual(materialize_output(case["expected"]["output"]), b"")
        with self.assertRaises(zlib.error):
            zlib.decompress(compressed, wbits=-15)

    def test_hdist_32_reserved_symbols_are_rejected_when_decoded(self) -> None:
        cases = {
            case["id"]: case
            for case in self.document["cases"]
            if case["id"].startswith("zip-raw-v1-error-dynamic-reserved-distance-")
        }
        self.assertEqual(
            set(cases),
            {
                "zip-raw-v1-error-dynamic-reserved-distance-30",
                "zip-raw-v1-error-dynamic-reserved-distance-31",
            },
        )
        for symbol in (30, 31):
            case = cases[f"zip-raw-v1-error-dynamic-reserved-distance-{symbol}"]
            compressed = bytes.fromhex(case["input_hex"])
            first_bits = int.from_bytes(compressed[:2], "little")
            self.assertEqual((first_bits >> 1) & 0b11, 0b10, case["id"])
            self.assertEqual((first_bits >> 8) & 0b11111, 31, case["id"])
            self.assertEqual(case["expected"]["error_id"], "reserved-distance-symbol")
            literal_lengths, distance_lengths, literal_symbol, distance_symbol = (
                decode_dynamic_prefix(compressed)
            )
            self.assertEqual(len(distance_lengths), 32)
            self.assertEqual(distance_lengths[symbol], 1)
            self.assertEqual(distance_lengths[31 if symbol == 30 else 30], 0)
            self.assertEqual(literal_symbol, 257)
            self.assertEqual(distance_symbol, symbol)
            with self.assertRaises(zlib.error, msg=case["id"]):
                zlib.decompress(compressed, wbits=-15)

    def test_malformed_wire_cases_are_rejected_by_the_foreign_oracle(self) -> None:
        limit_errors = {"invalid-output-limit", "output-limit-exceeded"}
        for case in self.document["cases"]:
            if case["operation"] != "inflate-error":
                continue
            if case["expected"]["error_id"] in limit_errors:
                continue
            with self.assertRaises(zlib.error, msg=case["id"]):
                zlib.decompress(bytes.fromhex(case["input_hex"]), wbits=-15)

    def test_deflate_cases_round_trip_through_independent_zlib(self) -> None:
        for case in self.document["cases"]:
            if case["operation"] != "deflate-interoperability":
                continue
            source = bytes.fromhex(case["input_hex"])
            expected = materialize_output(case["expected"]["output"])
            self.assertEqual(expected, source, case["id"])
            self.assertEqual(zlib.decompress(raw_deflate(source), wbits=-15), expected)

    def test_crc32_cases_match_the_independent_binascii_oracle(self) -> None:
        for case in self.document["cases"]:
            if case["operation"] != "crc32":
                continue
            initial = int(case.get("initial_crc32_hex", "00000000"), 16)
            actual = initial
            for chunk in case["chunks_hex"]:
                actual = binascii.crc32(bytes.fromhex(chunk), actual) & 0xFFFFFFFF
            self.assertEqual(f"{actual:08x}", case["expected"]["crc32_hex"], case["id"])


if __name__ == "__main__":
    unittest.main()
