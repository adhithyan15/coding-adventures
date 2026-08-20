"""Validate the language-neutral IC18 PNG corpus independently."""

from __future__ import annotations

import binascii
import importlib.util
import json
import struct
import unittest
import zlib
from copy import deepcopy
from pathlib import Path
from types import ModuleType
from typing import Any, cast

from jsonschema import Draft202012Validator  # type: ignore[import-untyped]

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = REPO_ROOT / "code/specs/fixtures/image-codec-png-v1"
SIGNATURE = b"\x89PNG\r\n\x1a\n"
EXPECTED_ERROR_IDS = (
    "invalid-max-pixels",
    "invalid-image-dimensions",
    "invalid-pixel-data-length",
    "file-too-short",
    "invalid-signature",
    "truncated-chunk",
    "invalid-chunk-type",
    "chunk-crc-mismatch",
    "chunk-before-ihdr",
    "duplicate-ihdr",
    "invalid-ihdr-length",
    "invalid-dimensions",
    "dimension-limit",
    "pixel-limit",
    "unsupported-feature",
    "invalid-plte",
    "invalid-trns",
    "nonconsecutive-idat",
    "invalid-iend",
    "trailing-data",
    "unknown-critical-chunk",
    "missing-required-chunk",
    "invalid-zlib-header",
    "preset-dictionary",
    "inflate-failed",
    "inflated-length-mismatch",
    "idat-cavity",
    "adler-mismatch",
    "invalid-filter",
)


def load_json(path: Path) -> dict[str, Any]:
    """Load JSON while rejecting duplicate object keys."""

    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    return cast(
        dict[str, Any],
        json.loads(path.read_text("utf-8"), object_pairs_hook=reject_duplicates),
    )


def fixture_document() -> dict[str, Any]:
    return load_json(FIXTURE_ROOT / "cases.json")


def require(condition: object, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def load_generator() -> ModuleType:
    path = FIXTURE_ROOT / "generate_cases.py"
    spec = importlib.util.spec_from_file_location("image_codec_png_cases", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load the fixture generator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def parse_chunks(png: bytes) -> list[tuple[str, bytes]]:
    require(png.startswith(SIGNATURE), "valid fixture lost the PNG signature")
    chunks: list[tuple[str, bytes]] = []
    offset = len(SIGNATURE)
    while offset < len(png):
        require(offset + 12 <= len(png), "valid fixture has a truncated chunk")
        length = struct.unpack_from(">I", png, offset)[0]
        type_bytes = png[offset + 4 : offset + 8]
        data_start = offset + 8
        data_end = data_start + length
        require(data_end + 4 <= len(png), "valid fixture chunk exceeds the file")
        require(
            all(
                ord("A") <= byte <= ord("Z") or ord("a") <= byte <= ord("z")
                for byte in type_bytes
            ),
            "valid fixture has a non-letter chunk type",
        )
        require(type_bytes[2] & 0x20 == 0, "valid fixture sets the reserved chunk bit")
        expected_crc = struct.unpack_from(">I", png, data_end)[0]
        require(
            binascii.crc32(type_bytes + png[data_start:data_end]) == expected_crc,
            "valid fixture has a bad chunk CRC",
        )
        chunks.append((type_bytes.decode("ascii"), png[data_start:data_end]))
        offset = data_end + 4
    require(offset == len(png), "valid fixture has trailing chunk bytes")
    return chunks


def paeth(a: int, b: int, c: int) -> int:
    estimate = a + b - c
    da, db, dc = abs(estimate - a), abs(estimate - b), abs(estimate - c)
    if da <= db and da <= dc:
        return a
    if db <= dc:
        return b
    return c


def unfilter(row: bytearray, prior: bytes, kind: int, bpp: int) -> None:
    for index in range(len(row)):
        left = row[index - bpp] if index >= bpp else 0
        above = prior[index]
        upper_left = prior[index - bpp] if index >= bpp else 0
        if kind == 0:
            predictor = 0
        elif kind == 1:
            predictor = left
        elif kind == 2:
            predictor = above
        elif kind == 3:
            predictor = (left + above) // 2
        elif kind == 4:
            predictor = paeth(left, above, upper_left)
        else:
            raise AssertionError(f"fixture success case has filter {kind}")
        row[index] = (row[index] + predictor) & 0xFF


def independent_decode(png: bytes) -> tuple[int, int, bytes, int]:
    chunks = parse_chunks(png)
    require(chunks[0][0] == "IHDR", "valid fixture does not start with IHDR")
    require(chunks[-1] == ("IEND", b""), "valid fixture does not end with empty IEND")
    require(
        [kind for kind, _ in chunks].count("IHDR") == 1,
        "valid fixture does not contain exactly one IHDR",
    )
    idat_indexes = [index for index, (kind, _) in enumerate(chunks) if kind == "IDAT"]
    require(idat_indexes, "valid fixture has no IDAT")
    require(
        idat_indexes == list(range(idat_indexes[0], idat_indexes[-1] + 1)),
        "valid fixture IDAT chunks are not consecutive",
    )

    header = chunks[0][1]
    require(len(header) == 13, "valid fixture IHDR is not 13 bytes")
    width, height = struct.unpack_from(">II", header)
    depth, colour, compression, filter_method, interlace = header[8:]
    require(depth == 8, "valid fixture depth is not 8")
    require(colour in (0, 2, 4, 6), "valid fixture colour type is unsupported")
    require(
        (compression, filter_method, interlace) == (0, 0, 0),
        "valid fixture has an unsupported IHDR method",
    )
    plte_chunks = [
        (index, data) for index, (kind, data) in enumerate(chunks) if kind == "PLTE"
    ]
    require(len(plte_chunks) <= 1, "valid fixture repeats PLTE")
    if plte_chunks:
        index, palette = plte_chunks[0]
        require(index < idat_indexes[0], "valid fixture puts PLTE after IDAT")
        require(colour in (2, 6), "valid fixture puts PLTE on a greyscale image")
        require(
            3 <= len(palette) <= 768 and len(palette) % 3 == 0,
            "valid fixture has an invalid PLTE length",
        )

    transparent_grey: int | None = None
    transparent_rgb: tuple[int, int, int] | None = None
    trns_chunks = [
        (index, data) for index, (kind, data) in enumerate(chunks) if kind == "tRNS"
    ]
    require(len(trns_chunks) <= 1, "valid fixture repeats tRNS")
    if trns_chunks:
        index, transparency = trns_chunks[0]
        require(index < idat_indexes[0], "valid fixture puts tRNS after IDAT")
        if plte_chunks:
            require(plte_chunks[0][0] < index, "valid fixture puts PLTE after tRNS")
        if colour == 0:
            require(len(transparency) == 2, "greyscale tRNS is not two bytes")
            transparent_grey = struct.unpack(">H", transparency)[0]
            require(transparent_grey <= 255, "greyscale tRNS exceeds 8-bit depth")
        elif colour == 2:
            require(len(transparency) == 6, "truecolour tRNS is not six bytes")
            transparent_rgb = struct.unpack(">HHH", transparency)
            require(max(transparent_rgb) <= 255, "truecolour tRNS exceeds 8-bit depth")
        else:
            raise AssertionError("valid fixture puts tRNS on a colour type with alpha")
    channels = {0: 1, 2: 3, 4: 2, 6: 4}[colour]
    stream = b"".join(data for kind, data in chunks if kind == "IDAT")
    require(len(stream) >= 6, "valid fixture has a short zlib stream")
    require(stream[0] & 0x0F == 8, "valid fixture zlib method is not DEFLATE")
    require(stream[0] >> 4 <= 7, "valid fixture zlib CINFO is too large")
    require((stream[0] << 8 | stream[1]) % 31 == 0, "valid fixture FCHECK is invalid")
    require(stream[1] & 0x20 == 0, "valid fixture requests a preset dictionary")
    inflater = zlib.decompressobj()
    filtered = inflater.decompress(stream) + inflater.flush()
    require(inflater.eof, "valid fixture zlib stream is incomplete")
    require(not inflater.unused_data, "valid fixture zlib stream has trailing bytes")
    require(not inflater.unconsumed_tail, "valid fixture zlib input was not consumed")
    require(
        zlib.adler32(filtered) == struct.unpack(">I", stream[-4:])[0],
        "valid fixture Adler-32 is incorrect",
    )

    stride = width * channels
    require(
        len(filtered) == height * (stride + 1),
        "valid fixture filtered size disagrees with IHDR",
    )
    prior = bytes(stride)
    rgba = bytearray()
    filter_mask = 0
    for y in range(height):
        start = y * (stride + 1)
        kind = filtered[start]
        filter_mask |= 1 << kind
        row = bytearray(filtered[start + 1 : start + 1 + stride])
        unfilter(row, prior, kind, channels)
        for x in range(width):
            pixel = row[x * channels : (x + 1) * channels]
            if colour == 0:
                alpha = 0 if pixel[0] == transparent_grey else 255
                rgba.extend([pixel[0], pixel[0], pixel[0], alpha])
            elif colour == 2:
                alpha = 0 if tuple(pixel) == transparent_rgb else 255
                rgba.extend([pixel[0], pixel[1], pixel[2], alpha])
            elif colour == 4:
                rgba.extend([pixel[0], pixel[0], pixel[0], pixel[1]])
            else:
                rgba.extend(pixel)
        prior = bytes(row)
    return width, height, bytes(rgba), filter_mask


def validate_encode_cases(document: dict[str, Any]) -> None:
    """Validate semantic constraints that JSON Schema cannot express."""

    encode_cases = [case for case in document["cases"] if case["operation"] == "encode"]
    require(encode_cases, "fixture has no successful encode case")
    for case in encode_cases:
        pixels = case["input"]
        width = pixels["width"]
        height = pixels["height"]
        require(
            type(width) is int and width > 0,
            f"{case['id']} width is not a positive integer",
        )
        require(
            type(height) is int and height > 0,
            f"{case['id']} height is not a positive integer",
        )
        require(
            len(bytes.fromhex(pixels["rgba_hex"])) == width * height * 4,
            f"{case['id']} RGBA length disagrees with its dimensions",
        )
        expected = case["expected"]
        require(
            set(expected)
            == {"chunk_types", "filter_types", "bit_depth", "colour_type", "interlace"},
            f"{case['id']} encode expectations are not closed",
        )
        require(
            expected["chunk_types"] == ["IHDR", "IDAT", "IEND"]
            and expected["bit_depth"] == 8
            and expected["colour_type"] == 6
            and expected["interlace"] == 0,
            f"{case['id']} does not pin the required encoded PNG profile",
        )
        require(
            len(expected["filter_types"]) == height
            and all(
                type(value) is int and 0 <= value <= 4
                for value in expected["filter_types"]
            ),
            f"{case['id']} does not pin one valid filter type per row",
        )


class ImageCodecPngFixtureTests(unittest.TestCase):
    def test_schema_closes_profile_limits_and_case_shapes(self) -> None:
        schema = load_json(FIXTURE_ROOT / "schema.json")
        document = fixture_document()
        Draft202012Validator.check_schema(schema)
        Draft202012Validator(schema).validate(document)
        self.assertEqual(document["schema_version"], 1)
        self.assertEqual(document["profile"], "image-codec-png-v1")
        self.assertEqual(
            document["limits"],
            {"max_dimension": 16384, "default_max_pixels": 33554432},
        )
        self.assertEqual(tuple(document["error_ids"]), EXPECTED_ERROR_IDS)

    def test_case_ids_are_unique_and_every_error_is_exercised(self) -> None:
        document = fixture_document()
        identifiers = [case["id"] for case in document["cases"]]
        self.assertEqual(len(identifiers), len(set(identifiers)))
        exercised = {
            case["expected"]["error_id"]
            for case in document["cases"]
            if case["operation"] in ("decode-error", "encode-error")
        }
        self.assertEqual(exercised, set(EXPECTED_ERROR_IDS))

    def test_encode_vectors_have_exact_pixel_and_chunk_contracts(self) -> None:
        schema = load_json(FIXTURE_ROOT / "schema.json")
        document = fixture_document()
        validate_encode_cases(document)

        empty_chunks = deepcopy(document)
        encode = next(
            case for case in empty_chunks["cases"] if case["operation"] == "encode"
        )
        encode["expected"]["chunk_types"] = []
        self.assertTrue(list(Draft202012Validator(schema).iter_errors(empty_chunks)))

        wrong_length = deepcopy(document)
        encode = next(
            case for case in wrong_length["cases"] if case["operation"] == "encode"
        )
        encode["input"]["rgba_hex"] = "00"
        with self.assertRaisesRegex(AssertionError, "RGBA length"):
            validate_encode_cases(wrong_length)

        fractional_width = deepcopy(document)
        encode = next(
            case for case in fractional_width["cases"] if case["operation"] == "encode"
        )
        encode["input"]["width"] = 1.5
        with self.assertRaisesRegex(AssertionError, "positive integer"):
            validate_encode_cases(fractional_width)

    def test_success_vectors_decode_with_independent_python_zlib(self) -> None:
        cases = [
            case
            for case in fixture_document()["cases"]
            if case["operation"] == "decode"
        ]
        masks: dict[str, int] = {}
        for case in cases:
            width, height, rgba, mask = independent_decode(
                bytes.fromhex(case["png_hex"])
            )
            expected = case["expected"]
            self.assertEqual(
                (width, height), (expected["width"], expected["height"]), case["id"]
            )
            self.assertEqual(rgba.hex(), expected["rgba_hex"], case["id"])
            masks[case["id"]] = mask
        self.assertEqual(masks["png-v1-decode-all-filters"] & 0b11111, 0b11111)

    def test_paeth_vectors_pin_all_predictors_and_ties(self) -> None:
        identifiers = {case["id"] for case in fixture_document()["cases"]}
        self.assertTrue(
            {
                "png-v1-decode-paeth-a",
                "png-v1-decode-paeth-b",
                "png-v1-decode-paeth-c",
                "png-v1-decode-paeth-a-b-tie",
                "png-v1-decode-paeth-all-tie",
            }.issubset(identifiers)
        )

    def test_corpus_contains_stored_fixed_and_dynamic_deflate(self) -> None:
        expected = {
            "png-v1-decode-stored-deflate": 0,
            "png-v1-decode-fixed-deflate": 1,
            "png-v1-decode-dynamic-deflate": 2,
        }
        by_id = {case["id"]: case for case in fixture_document()["cases"]}
        for identifier, block_type in expected.items():
            chunks = parse_chunks(bytes.fromhex(by_id[identifier]["png_hex"]))
            stream = b"".join(data for kind, data in chunks if kind == "IDAT")
            self.assertEqual((stream[2] >> 1) & 0x03, block_type)

    def test_generator_is_deterministic_and_fixture_is_bounded(self) -> None:
        module = load_generator()
        rendered = module.rendered()
        self.assertEqual((FIXTURE_ROOT / "cases.json").read_text("utf-8"), rendered)
        document = fixture_document()
        self.assertGreaterEqual(len(document["cases"]), 40)
        self.assertLess(len(rendered.encode("utf-8")), 256 * 1024)
        for case in document["cases"]:
            if "png_hex" in case:
                self.assertLessEqual(len(case["png_hex"]) // 2, 1024 * 1024)

    def test_schema_rejects_unknown_fields_and_duplicate_case_shapes(self) -> None:
        schema = load_json(FIXTURE_ROOT / "schema.json")
        document = deepcopy(fixture_document())
        document["unexpected"] = True
        self.assertTrue(list(Draft202012Validator(schema).iter_errors(document)))

        taxonomy = deepcopy(fixture_document())
        taxonomy["error_ids"] = [f"arbitrary-code-{index}" for index in range(29)]
        self.assertTrue(list(Draft202012Validator(schema).iter_errors(taxonomy)))


if __name__ == "__main__":
    unittest.main()
