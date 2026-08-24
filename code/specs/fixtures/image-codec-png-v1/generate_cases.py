"""Generate deterministic IC18 PNG vectors without host-zlib byte choices."""

from __future__ import annotations

import argparse
import binascii
import itertools
import json
import struct
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent
SIGNATURE = b"\x89PNG\r\n\x1a\n"
MAX_PIXELS = 32 * 1024 * 1024

ERROR_IDS = [
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
]

# Checked RFC 1951 dynamic-Huffman vector shared with the ZIP neutral corpus.
# Keeping the raw bytes here avoids asking a host zlib version to choose a block
# strategy, which is deliberately not stable across zlib releases.
DYNAMIC_RAW = bytes.fromhex(
    "0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e"
)
DYNAMIC_FILTERED = bytes.fromhex(
    "0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201"
)


def u32(value: int) -> bytes:
    return struct.pack(">I", value)


def chunk(kind: bytes, data: bytes) -> bytes:
    return u32(len(data)) + kind + data + u32(binascii.crc32(kind + data))


def ihdr(
    width: int,
    height: int,
    *,
    depth: int = 8,
    colour: int = 6,
    compression: int = 0,
    filter_method: int = 0,
    interlace: int = 0,
) -> bytes:
    return chunk(
        b"IHDR",
        u32(width)
        + u32(height)
        + bytes([depth, colour, compression, filter_method, interlace]),
    )


def paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def filter_row(kind: int, raw: bytes, prior: bytes, bpp: int) -> bytes:
    out = bytearray(len(raw))
    for index, value in enumerate(raw):
        left = raw[index - bpp] if index >= bpp else 0
        above = prior[index]
        upper_left = prior[index - bpp] if index >= bpp else 0
        predictor = {
            0: 0,
            1: left,
            2: above,
            3: (left + above) // 2,
            4: paeth(left, above, upper_left),
        }[kind]
        out[index] = (value - predictor) & 0xFF
    return bytes([kind]) + bytes(out)


def filtered_rows(rows: list[bytes], filters: list[int], bpp: int) -> bytes:
    prior = bytes(len(rows[0]))
    output = bytearray()
    for raw, kind in zip(rows, filters, strict=True):
        output.extend(filter_row(kind, raw, prior, bpp))
        prior = raw
    return bytes(output)


def choose_filter_types(rows: list[bytes], bpp: int) -> list[int]:
    """Apply IC18's signed-byte score and lowest-number tie break."""

    prior = bytes(len(rows[0]))
    result = []
    for raw in rows:
        candidates = [filter_row(kind, raw, prior, bpp) for kind in range(5)]
        scores = [
            sum(abs(value if value < 128 else value - 256) for value in row[1:])
            for row in candidates
        ]
        result.append(min(range(5), key=scores.__getitem__))
        prior = raw
    return result


def greyscale_rgba(rows: list[bytes]) -> bytes:
    return b"".join(bytes([value, value, value, 255]) for row in rows for value in row)


def adler32(data: bytes) -> int:
    """Return RFC 1950 Adler-32 without depending on a host zlib build."""

    first = 1
    second = 0
    for value in data:
        first = (first + value) % 65521
        second = (second + first) % 65521
    return (second << 16) | first


def zlib_stream(raw_deflate: bytes, decoded: bytes) -> bytes:
    """Wrap deterministic RFC 1951 bytes in an RFC 1950 stream."""

    return b"\x78\x01" + raw_deflate + u32(adler32(decoded))


def stored_raw(data: bytes) -> bytes:
    """Encode deterministic stored RFC 1951 blocks."""

    output = bytearray()
    chunks = [data[index : index + 65535] for index in range(0, len(data), 65535)]
    if not chunks:
        chunks = [b""]
    for index, value in enumerate(chunks):
        output.append(1 if index == len(chunks) - 1 else 0)
        output.extend(struct.pack("<H", len(value)))
        output.extend(struct.pack("<H", (~len(value)) & 0xFFFF))
        output.extend(value)
    return bytes(output)


def reverse_bits(value: int, width: int) -> int:
    result = 0
    for _ in range(width):
        result = (result << 1) | (value & 1)
        value >>= 1
    return result


def fixed_code(symbol: int) -> tuple[int, int]:
    if symbol <= 143:
        return 0x30 + symbol, 8
    if symbol <= 255:
        return 0x190 + symbol - 144, 9
    if symbol <= 279:
        return symbol - 256, 7
    return 0xC0 + symbol - 280, 8


def fixed_literal_raw(data: bytes) -> bytes:
    """Encode one final fixed-Huffman block using literal symbols only."""

    output = bytearray()
    bit_buffer = 0
    bit_count = 0

    def write_bits(value: int, width: int) -> None:
        nonlocal bit_buffer, bit_count
        bit_buffer |= value << bit_count
        bit_count += width
        while bit_count >= 8:
            output.append(bit_buffer & 0xFF)
            bit_buffer >>= 8
            bit_count -= 8

    write_bits(0b011, 3)  # BFINAL=1, BTYPE=01 (fixed Huffman).
    for symbol in [*data, 256]:
        code, width = fixed_code(symbol)
        write_bits(reverse_bits(code, width), width)
    if bit_count:
        output.append(bit_buffer)
    return bytes(output)


def fixed_zlib(data: bytes) -> bytes:
    return zlib_stream(fixed_literal_raw(data), data)


def stored_zlib(data: bytes) -> bytes:
    return zlib_stream(stored_raw(data), data)


def png_from_zlib(
    width: int,
    height: int,
    zstream: bytes,
    *,
    depth: int = 8,
    colour: int = 6,
    compression: int = 0,
    filter_method: int = 0,
    interlace: int = 0,
    split_at: list[int] | None = None,
    between_idat: bytes | None = None,
    after_ihdr: bytes = b"",
    before_iend: bytes = b"",
    iend_data: bytes = b"",
    trailing: bytes = b"",
) -> bytes:
    header = ihdr(
        width,
        height,
        depth=depth,
        colour=colour,
        compression=compression,
        filter_method=filter_method,
        interlace=interlace,
    )
    boundaries = [0, *(split_at or []), len(zstream)]
    idats = []
    for index, (start, end) in enumerate(itertools.pairwise(boundaries)):
        idats.append(chunk(b"IDAT", zstream[start:end]))
        if between_idat is not None and index == 0:
            idats.append(between_idat)
    return (
        SIGNATURE
        + header
        + after_ihdr
        + b"".join(idats)
        + before_iend
        + chunk(b"IEND", iend_data)
        + trailing
    )


def png_from_rows(
    width: int,
    rows: list[bytes],
    filters: list[int],
    *,
    colour: int,
    bpp: int,
    **kwargs: Any,
) -> bytes:
    filtered = filtered_rows(rows, filters, bpp)
    return png_from_zlib(
        width, len(rows), fixed_zlib(filtered), colour=colour, **kwargs
    )


def valid_flg(cmf: int, *, dictionary: bool = False) -> int:
    base = 0x20 if dictionary else 0
    for check in range(32):
        value = base | check
        if ((cmf << 8) | value) % 31 == 0:
            return value
    raise AssertionError("FCHECK has no solution")


def decode_case(
    identifier: str,
    png: bytes,
    width: int,
    height: int,
    rgba: bytes,
    *,
    oracle: str = "python-zlib",
    max_pixels: int | None = None,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "id": identifier,
        "operation": "decode",
        "png_hex": png.hex(),
        "oracle": oracle,
        "expected": {"width": width, "height": height, "rgba_hex": rgba.hex()},
    }
    if max_pixels is not None:
        result["options"] = {"max_pixels": max_pixels}
    return result


def decode_error(
    identifier: str,
    png: bytes,
    error_id: str,
    *,
    max_pixels: float | None = None,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "id": identifier,
        "operation": "decode-error",
        "png_hex": png.hex(),
        "expected": {"error_id": error_id},
    }
    if max_pixels is not None:
        result["options"] = {"max_pixels": max_pixels}
    return result


def document() -> dict[str, Any]:
    rgba_one = bytes([1, 2, 3, 4])
    rgba_one_png = png_from_rows(1, [rgba_one], [0], colour=6, bpp=4)
    rows = [
        bytes([10 + y, 20 + y, 30 + y, 40 + y, 50 + y, 60 + y, 70 + y, 80 + y])
        for y in range(5)
    ]
    all_filters = png_from_rows(2, rows, [0, 1, 2, 3, 4], colour=6, bpp=4)
    paeth_b_rows = [bytes([50]), bytes([7])]
    paeth_a_rows = [bytes([100, 100]), bytes([10, 7])]
    paeth_c_rows = [bytes([15, 20]), bytes([10, 7])]
    paeth_ab_tie_rows = [bytes([0, 10]), bytes([10, 7])]
    paeth_all_tie_rows = [bytes([10, 10]), bytes([10, 7])]
    grey = bytes([17, 200])
    grey_png = png_from_rows(2, [grey], [1], colour=0, bpp=1)
    grey_rgba = bytes([17, 17, 17, 255, 200, 200, 200, 255])
    rgb = bytes([10, 20, 30, 40, 50, 60])
    rgb_png = png_from_rows(2, [rgb], [0], colour=2, bpp=3)
    rgb_rgba = bytes([10, 20, 30, 255, 40, 50, 60, 255])
    ga = bytes([70, 80, 90, 100])
    ga_png = png_from_rows(2, [ga], [0], colour=4, bpp=2)
    ga_rgba = bytes([70, 70, 70, 80, 90, 90, 90, 100])
    grey_trns = bytes([17, 18])
    grey_trns_png = png_from_rows(
        2,
        [grey_trns],
        [0],
        colour=0,
        bpp=1,
        after_ihdr=chunk(b"tRNS", bytes([0, 17])),
    )
    grey_trns_rgba = bytes([17, 17, 17, 0, 18, 18, 18, 255])
    rgb_trns = bytes([1, 2, 3, 4, 5, 6])
    rgb_trns_png = png_from_rows(
        2,
        [rgb_trns],
        [0],
        colour=2,
        bpp=3,
        after_ihdr=chunk(b"tRNS", bytes([0, 1, 0, 2, 0, 3])),
    )
    rgb_trns_rgba = bytes([1, 2, 3, 0, 4, 5, 6, 255])
    rgb_plte_trns_png = png_from_rows(
        2,
        [rgb_trns],
        [0],
        colour=2,
        bpp=3,
        after_ihdr=(
            chunk(b"PLTE", bytes([9, 8, 7])) + chunk(b"tRNS", bytes([0, 1, 0, 2, 0, 3]))
        ),
    )
    filtered_one = bytes([0]) + rgba_one
    z_one = fixed_zlib(filtered_one)
    stored_z = stored_zlib(filtered_one)
    fixed_z = fixed_zlib(filtered_one)
    dynamic_z = zlib_stream(DYNAMIC_RAW, DYNAMIC_FILTERED)
    dynamic_grey = bytearray()
    for value in DYNAMIC_FILTERED[1:]:
        left = dynamic_grey[-1] if dynamic_grey else 0
        dynamic_grey.append((value + left) & 0xFF)
    dynamic_rgba = b"".join(bytes([value, value, value, 255]) for value in dynamic_grey)
    encode_filter_rows = [
        bytes.fromhex(value)
        for value in (
            "0ceaad0dd4010353",
            "de4215549a044c75",
            "e328c65b05bc6e3e",
            "3d4c28fd54ab7668",
            "5d7c5e0dbea95aca",
        )
    ]

    cases: list[dict[str, Any]] = [
        decode_case("png-v1-decode-rgba-none", rgba_one_png, 1, 1, rgba_one),
        decode_case(
            "png-v1-decode-all-filters",
            all_filters,
            2,
            5,
            b"".join(rows),
        ),
        decode_case(
            "png-v1-decode-paeth-b",
            png_from_rows(1, paeth_b_rows, [0, 4], colour=0, bpp=1),
            1,
            2,
            greyscale_rgba(paeth_b_rows),
        ),
        decode_case(
            "png-v1-decode-paeth-a",
            png_from_rows(2, paeth_a_rows, [0, 4], colour=0, bpp=1),
            2,
            2,
            greyscale_rgba(paeth_a_rows),
        ),
        decode_case(
            "png-v1-decode-paeth-c",
            png_from_rows(2, paeth_c_rows, [0, 4], colour=0, bpp=1),
            2,
            2,
            greyscale_rgba(paeth_c_rows),
        ),
        decode_case(
            "png-v1-decode-paeth-a-b-tie",
            png_from_rows(2, paeth_ab_tie_rows, [0, 4], colour=0, bpp=1),
            2,
            2,
            greyscale_rgba(paeth_ab_tie_rows),
        ),
        decode_case(
            "png-v1-decode-paeth-all-tie",
            png_from_rows(2, paeth_all_tie_rows, [0, 4], colour=0, bpp=1),
            2,
            2,
            greyscale_rgba(paeth_all_tie_rows),
        ),
        decode_case("png-v1-decode-greyscale", grey_png, 2, 1, grey_rgba),
        decode_case("png-v1-decode-truecolour", rgb_png, 2, 1, rgb_rgba),
        decode_case("png-v1-decode-greyscale-alpha", ga_png, 2, 1, ga_rgba),
        decode_case(
            "png-v1-decode-greyscale-trns",
            grey_trns_png,
            2,
            1,
            grey_trns_rgba,
            oracle="rfc2083-hand",
        ),
        decode_case(
            "png-v1-decode-truecolour-trns",
            rgb_trns_png,
            2,
            1,
            rgb_trns_rgba,
            oracle="rfc2083-hand",
        ),
        decode_case(
            "png-v1-decode-truecolour-plte-trns",
            rgb_plte_trns_png,
            2,
            1,
            rgb_trns_rgba,
            oracle="rfc2083-hand",
        ),
        decode_case(
            "png-v1-decode-split-idat",
            png_from_zlib(1, 1, z_one, split_at=[1, 3, len(z_one) - 1]),
            1,
            1,
            rgba_one,
        ),
        decode_case(
            "png-v1-decode-ancillary",
            png_from_zlib(1, 1, z_one, after_ihdr=chunk(b"tEXt", b"fixture")),
            1,
            1,
            rgba_one,
        ),
        decode_case(
            "png-v1-decode-suggested-plte",
            png_from_zlib(
                1,
                1,
                z_one,
                after_ihdr=chunk(b"PLTE", bytes([9, 8, 7])),
            ),
            1,
            1,
            rgba_one,
            oracle="rfc2083-hand",
        ),
        decode_case(
            "png-v1-decode-lowered-limit",
            rgba_one_png,
            1,
            1,
            rgba_one,
            max_pixels=1,
        ),
        decode_case(
            "png-v1-decode-stored-deflate",
            png_from_zlib(1, 1, stored_z),
            1,
            1,
            rgba_one,
        ),
        decode_case(
            "png-v1-decode-fixed-deflate",
            png_from_zlib(1, 1, fixed_z),
            1,
            1,
            rgba_one,
        ),
        decode_case(
            "png-v1-decode-dynamic-deflate",
            png_from_zlib(63, 1, dynamic_z, colour=0),
            63,
            1,
            dynamic_rgba,
        ),
        {
            "id": "png-v1-encode-single-rgba",
            "operation": "encode",
            "input": {"width": 1, "height": 1, "rgba_hex": rgba_one.hex()},
            "oracle": "foreign-zlib",
            "expected": {
                "chunk_types": ["IHDR", "IDAT", "IEND"],
                "filter_types": choose_filter_types([rgba_one], 4),
                "bit_depth": 8,
                "colour_type": 6,
                "interlace": 0,
            },
        },
        {
            "id": "png-v1-encode-two-by-two",
            "operation": "encode",
            "input": {
                "width": 2,
                "height": 2,
                "rgba_hex": bytes(range(16)).hex(),
            },
            "oracle": "foreign-zlib",
            "expected": {
                "chunk_types": ["IHDR", "IDAT", "IEND"],
                "filter_types": choose_filter_types(
                    [bytes(range(8)), bytes(range(8, 16))], 4
                ),
                "bit_depth": 8,
                "colour_type": 6,
                "interlace": 0,
            },
        },
        {
            "id": "png-v1-encode-all-filters",
            "operation": "encode",
            "input": {
                "width": 2,
                "height": 5,
                "rgba_hex": b"".join(encode_filter_rows).hex(),
            },
            "oracle": "foreign-zlib",
            "expected": {
                "chunk_types": ["IHDR", "IDAT", "IEND"],
                "filter_types": choose_filter_types(encode_filter_rows, 4),
                "bit_depth": 8,
                "colour_type": 6,
                "interlace": 0,
            },
        },
        {
            "id": "png-v1-adler-wikipedia",
            "operation": "adler32",
            "input_hex": b"Wikipedia".hex(),
            "expected": {"adler32_hex": "11e60398"},
        },
        {
            "id": "png-v1-adler-chunk-boundary",
            "operation": "adler32",
            "input_hex": (bytes(range(256)) * 22).hex(),
            "expected": {"adler32_hex": f"{adler32(bytes(range(256)) * 22):08x}"},
        },
    ]

    signature_only = SIGNATURE
    valid_parts = SIGNATURE + ihdr(1, 1) + chunk(b"IDAT", z_one) + chunk(b"IEND", b"")
    bad_crc = bytearray(valid_parts)
    bad_crc[20] ^= 1
    idat_cavity = z_one[:2] + z_one[2:-4] + b"CAVITY" + z_one[-4:]
    bad_adler = bytearray(z_one)
    bad_adler[-1] ^= 1
    short_filtered = fixed_zlib(bytes([0, 1, 2, 3]))
    long_filtered = fixed_zlib(bytes([0, 1, 2, 3, 4, 5]))
    invalid_filter = fixed_zlib(bytes([5, 1, 2, 3, 4]))

    cases.extend(
        [
            decode_error("png-v1-error-too-short", b"\x89PNG", "file-too-short"),
            decode_error(
                "png-v1-error-signature",
                bytes([0]) + SIGNATURE[1:],
                "invalid-signature",
            ),
            decode_error(
                "png-v1-error-truncated-header",
                SIGNATURE + b"\x00\x00\x00\x0d",
                "truncated-chunk",
            ),
            decode_error(
                "png-v1-error-truncated-data",
                SIGNATURE + u32(13) + b"IHDR" + b"\x00" * 5,
                "truncated-chunk",
            ),
            decode_error("png-v1-error-crc", bytes(bad_crc), "chunk-crc-mismatch"),
            decode_error(
                "png-v1-error-chunk-type-character",
                png_from_zlib(1, 1, z_one, after_ihdr=chunk(b"1BAD", b"")),
                "invalid-chunk-type",
            ),
            decode_error(
                "png-v1-error-chunk-type-reserved-bit",
                png_from_zlib(1, 1, z_one, after_ihdr=chunk(b"ABcD", b"")),
                "invalid-chunk-type",
            ),
            decode_error(
                "png-v1-error-before-ihdr",
                SIGNATURE + chunk(b"tEXt", b"x") + valid_parts[len(SIGNATURE) :],
                "chunk-before-ihdr",
            ),
            decode_error(
                "png-v1-error-duplicate-ihdr",
                SIGNATURE
                + ihdr(1, 1)
                + ihdr(1, 1)
                + chunk(b"IDAT", z_one)
                + chunk(b"IEND", b""),
                "duplicate-ihdr",
            ),
            decode_error(
                "png-v1-error-ihdr-length",
                SIGNATURE + chunk(b"IHDR", b"\x00" * 12) + chunk(b"IEND", b""),
                "invalid-ihdr-length",
            ),
            decode_error(
                "png-v1-error-zero-dimension",
                png_from_zlib(0, 1, z_one),
                "invalid-dimensions",
            ),
            decode_error(
                "png-v1-error-edge-limit",
                png_from_zlib(16385, 1, z_one),
                "dimension-limit",
            ),
            decode_error(
                "png-v1-error-pixel-limit",
                png_from_zlib(16384, 16384, z_one),
                "pixel-limit",
            ),
            decode_error(
                "png-v1-error-compression-method",
                png_from_zlib(1, 1, z_one, compression=1),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-filter-method",
                png_from_zlib(1, 1, z_one, filter_method=1),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-interlace",
                png_from_zlib(1, 1, z_one, interlace=1),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-palette",
                png_from_zlib(1, 1, z_one, colour=3),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-colour-type",
                png_from_zlib(1, 1, z_one, colour=5),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-bit-depth",
                png_from_zlib(1, 1, z_one, depth=16),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-apng-actl",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    after_ihdr=chunk(b"acTL", u32(1) + u32(0)),
                ),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-apng-fctl",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    after_ihdr=chunk(
                        b"fcTL",
                        u32(0)
                        + u32(1)
                        + u32(1)
                        + u32(0)
                        + u32(0)
                        + struct.pack(">HHBB", 1, 100, 0, 0),
                    ),
                ),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-apng-fdat",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    before_iend=chunk(b"fdAT", u32(1) + z_one),
                ),
                "unsupported-feature",
            ),
            decode_error(
                "png-v1-error-plte-greyscale",
                png_from_zlib(
                    1,
                    1,
                    fixed_zlib(bytes([0, 17])),
                    colour=0,
                    after_ihdr=chunk(b"PLTE", bytes([1, 2, 3])),
                ),
                "invalid-plte",
            ),
            decode_error(
                "png-v1-error-plte-length",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    after_ihdr=chunk(b"PLTE", bytes([1, 2])),
                ),
                "invalid-plte",
            ),
            decode_error(
                "png-v1-error-plte-incomplete-entry",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    after_ihdr=chunk(b"PLTE", bytes([1, 2, 3, 4])),
                ),
                "invalid-plte",
            ),
            decode_error(
                "png-v1-error-plte-entry-cap",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    after_ihdr=chunk(b"PLTE", bytes(771)),
                ),
                "invalid-plte",
            ),
            decode_error(
                "png-v1-error-plte-duplicate",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    after_ihdr=(
                        chunk(b"PLTE", bytes([1, 2, 3]))
                        + chunk(b"PLTE", bytes([4, 5, 6]))
                    ),
                ),
                "invalid-plte",
            ),
            decode_error(
                "png-v1-error-plte-after-idat",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    before_iend=chunk(b"PLTE", bytes([1, 2, 3])),
                ),
                "invalid-plte",
            ),
            decode_error(
                "png-v1-error-plte-after-trns",
                png_from_zlib(
                    1,
                    1,
                    fixed_zlib(bytes([0, 1, 2, 3])),
                    colour=2,
                    after_ihdr=(
                        chunk(b"tRNS", bytes([0, 1, 0, 2, 0, 3]))
                        + chunk(b"PLTE", bytes([9, 8, 7]))
                    ),
                ),
                "invalid-plte",
            ),
            decode_error(
                "png-v1-error-trns-alpha-colour",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    after_ihdr=chunk(b"tRNS", bytes([0, 1, 0, 2, 0, 3])),
                ),
                "invalid-trns",
            ),
            decode_error(
                "png-v1-error-trns-length",
                png_from_zlib(
                    1,
                    1,
                    fixed_zlib(bytes([0, 17])),
                    colour=0,
                    after_ihdr=chunk(b"tRNS", bytes([17])),
                ),
                "invalid-trns",
            ),
            decode_error(
                "png-v1-error-trns-truecolour-length",
                png_from_zlib(
                    1,
                    1,
                    fixed_zlib(bytes([0, 1, 2, 3])),
                    colour=2,
                    after_ihdr=chunk(b"tRNS", bytes([0, 1, 0, 2])),
                ),
                "invalid-trns",
            ),
            decode_error(
                "png-v1-error-trns-duplicate",
                png_from_zlib(
                    1,
                    1,
                    fixed_zlib(bytes([0, 17])),
                    colour=0,
                    after_ihdr=(
                        chunk(b"tRNS", bytes([0, 17])) + chunk(b"tRNS", bytes([0, 18]))
                    ),
                ),
                "invalid-trns",
            ),
            decode_error(
                "png-v1-error-trns-after-idat",
                png_from_zlib(
                    1,
                    1,
                    fixed_zlib(bytes([0, 17])),
                    colour=0,
                    before_iend=chunk(b"tRNS", bytes([0, 17])),
                ),
                "invalid-trns",
            ),
            decode_error(
                "png-v1-error-trns-sample-range",
                png_from_zlib(
                    1,
                    1,
                    fixed_zlib(bytes([0, 17])),
                    colour=0,
                    after_ihdr=chunk(b"tRNS", bytes([1, 0])),
                ),
                "invalid-trns",
            ),
            decode_error(
                "png-v1-error-trns-truecolour-sample-range",
                png_from_zlib(
                    1,
                    1,
                    fixed_zlib(bytes([0, 1, 2, 3])),
                    colour=2,
                    after_ihdr=chunk(b"tRNS", bytes([1, 0, 0, 2, 0, 3])),
                ),
                "invalid-trns",
            ),
            decode_error(
                "png-v1-error-idat-order",
                png_from_zlib(
                    1,
                    1,
                    z_one,
                    split_at=[3],
                    between_idat=chunk(b"tEXt", b"gap"),
                ),
                "nonconsecutive-idat",
            ),
            decode_error(
                "png-v1-error-iend-payload",
                png_from_zlib(1, 1, z_one, iend_data=b"x"),
                "invalid-iend",
            ),
            decode_error(
                "png-v1-error-after-iend",
                png_from_zlib(1, 1, z_one, trailing=b"passenger"),
                "trailing-data",
            ),
            decode_error(
                "png-v1-error-critical-chunk",
                png_from_zlib(1, 1, z_one, after_ihdr=chunk(b"ABCD", b"")),
                "unknown-critical-chunk",
            ),
            decode_error(
                "png-v1-error-missing-ihdr",
                signature_only,
                "missing-required-chunk",
            ),
            decode_error(
                "png-v1-error-missing-iend",
                SIGNATURE + ihdr(1, 1) + chunk(b"IDAT", z_one),
                "missing-required-chunk",
            ),
            decode_error(
                "png-v1-error-missing-idat",
                SIGNATURE + ihdr(1, 1) + chunk(b"IEND", b""),
                "missing-required-chunk",
            ),
            decode_error(
                "png-v1-error-zlib-short",
                png_from_zlib(1, 1, b"\x78\x9c\x00"),
                "invalid-zlib-header",
            ),
            decode_error(
                "png-v1-error-zlib-method",
                png_from_zlib(
                    1,
                    1,
                    bytes([0x79, valid_flg(0x79)]) + z_one[2:],
                ),
                "invalid-zlib-header",
            ),
            decode_error(
                "png-v1-error-zlib-cinfo",
                png_from_zlib(
                    1,
                    1,
                    bytes([0x88, valid_flg(0x88)]) + z_one[2:],
                ),
                "invalid-zlib-header",
            ),
            decode_error(
                "png-v1-error-zlib-fcheck",
                png_from_zlib(1, 1, bytes([z_one[0], z_one[1] ^ 1]) + z_one[2:]),
                "invalid-zlib-header",
            ),
            decode_error(
                "png-v1-error-zlib-dictionary",
                png_from_zlib(
                    1,
                    1,
                    bytes([0x78, valid_flg(0x78, dictionary=True)]) + z_one[2:],
                ),
                "preset-dictionary",
            ),
            decode_error(
                "png-v1-error-inflate",
                png_from_zlib(1, 1, b"\x78\x9c\x07\x00\x00\x00\x01"),
                "inflate-failed",
            ),
            decode_error(
                "png-v1-error-inflated-length",
                png_from_zlib(1, 1, short_filtered),
                "inflated-length-mismatch",
            ),
            decode_error(
                "png-v1-error-inflated-over-limit",
                png_from_zlib(1, 1, long_filtered),
                "inflated-length-mismatch",
            ),
            decode_error(
                "png-v1-error-idat-cavity",
                png_from_zlib(1, 1, idat_cavity),
                "idat-cavity",
            ),
            decode_error(
                "png-v1-error-adler",
                png_from_zlib(1, 1, bytes(bad_adler)),
                "adler-mismatch",
            ),
            decode_error(
                "png-v1-error-filter",
                png_from_zlib(1, 1, invalid_filter),
                "invalid-filter",
            ),
            decode_error(
                "png-v1-error-max-pixels-zero",
                rgba_one_png,
                "invalid-max-pixels",
                max_pixels=0,
            ),
            decode_error(
                "png-v1-error-max-pixels-fractional",
                rgba_one_png,
                "invalid-max-pixels",
                max_pixels=1.5,
            ),
            decode_error(
                "png-v1-error-max-pixels-raised",
                rgba_one_png,
                "invalid-max-pixels",
                max_pixels=MAX_PIXELS + 1,
            ),
            {
                "id": "png-v1-error-encode-empty",
                "operation": "encode-error",
                "input": {"width": 0, "height": 1, "rgba_hex": ""},
                "expected": {"error_id": "invalid-image-dimensions"},
            },
            {
                "id": "png-v1-error-encode-fractional",
                "operation": "encode-error",
                "input": {"width": 1.5, "height": 1, "rgba_hex": ""},
                "expected": {"error_id": "invalid-image-dimensions"},
            },
            {
                "id": "png-v1-error-encode-data-length",
                "operation": "encode-error",
                "input": {"width": 1, "height": 1, "rgba_hex": "010203"},
                "expected": {"error_id": "invalid-pixel-data-length"},
            },
        ]
    )

    return {
        "schema_version": 1,
        "profile": "image-codec-png-v1",
        "limits": {"max_dimension": 16384, "default_max_pixels": MAX_PIXELS},
        "error_ids": ERROR_IDS,
        "cases": cases,
    }


def rendered() -> str:
    return json.dumps(document(), indent=2, ensure_ascii=True) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    target = ROOT / "cases.json"
    expected = rendered()
    if args.check:
        return 0 if target.read_text("utf-8") == expected else 1
    target.write_text(expected, "utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
