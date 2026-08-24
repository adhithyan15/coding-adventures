from __future__ import annotations

import binascii
import math
import struct
from collections.abc import Callable

import pytest
from pixel_container import ImageCodec, PixelContainer  # type: ignore[import-untyped]

from image_codec_png import (
    PNG_ERROR_CODES,
    PNG_MAX_DIMENSION,
    PNG_MAX_PIXELS,
    PngCodec,
    PngError,
    adler32,
    decode_png,
    encode_png,
)


def _error(expected: str, action: Callable[[], object]) -> None:
    with pytest.raises(PngError) as raised:
        action()
    assert raised.value.code == expected
    assert raised.value.args == (expected,)


def _pixels(width: object, height: object, data: bytes) -> PixelContainer:
    result = object.__new__(PixelContainer)
    object.__setattr__(result, "width", width)
    object.__setattr__(result, "height", height)
    result.data = bytearray(data)
    return result


def _chunk(chunk_type: bytes, payload: bytes) -> bytes:
    checksum = binascii.crc32(chunk_type)
    checksum = binascii.crc32(payload, checksum)
    return struct.pack(">I", len(payload)) + chunk_type + payload + struct.pack(">I", checksum)


def _insert(png: bytes, offset: int, chunk: bytes) -> bytes:
    return png[:offset] + chunk + png[offset:]


def test_codec_contract_and_deterministic_round_trip() -> None:
    codec: ImageCodec = PngCodec()
    pixels = PixelContainer(2, 1, bytearray([1, 2, 3, 4, 5, 6, 7, 8]))
    first = codec.encode(pixels)
    second = encode_png(pixels)
    assert codec.mime_type == "image/png"
    assert first == second
    assert codec.decode(first) == pixels
    assert PngCodec(max_pixels=2).decode(first) == pixels


@pytest.mark.parametrize(
    "value",
    [0, -1, 1.5, PNG_MAX_PIXELS + 1, math.nan, math.inf, -math.inf, True, "1"],
)
def test_validates_max_pixels_before_parsing(value: object) -> None:
    _error("invalid-max-pixels", lambda: PngCodec(max_pixels=value))
    _error("invalid-max-pixels", lambda: decode_png(b"", max_pixels=value))


@pytest.mark.parametrize(
    ("width", "height"),
    [(0, 1), (1, 0), (1.5, 1), (True, 1), (-1, 1), (PNG_MAX_DIMENSION + 1, 1)],
)
def test_rejects_invalid_encoder_dimensions(width: object, height: object) -> None:
    _error("invalid-image-dimensions", lambda: encode_png(_pixels(width, height, b"")))


def test_rejects_encoder_pixel_product_and_wrong_length() -> None:
    _error(
        "invalid-image-dimensions",
        lambda: encode_png(_pixels(PNG_MAX_DIMENSION, PNG_MAX_DIMENSION, b"")),
    )
    _error("invalid-pixel-data-length", lambda: encode_png(_pixels(1, 1, b"\x00\x01\x02")))


def test_publishes_closed_payload_blind_taxonomy() -> None:
    assert isinstance(PNG_ERROR_CODES, tuple)
    assert len(PNG_ERROR_CODES) == 29
    error = PngError("invalid-filter")
    assert error.code == "invalid-filter"
    assert str(error) == "invalid-filter"


def test_apng_obeys_crc_and_first_ihdr_precedence() -> None:
    encoded = encode_png(PixelContainer(1, 1, bytearray(4)))
    valid = _chunk(b"acTL", bytes(8))
    _error("unsupported-feature", lambda: decode_png(_insert(encoded, 33, valid)))
    corrupt = valid[:-1] + bytes([valid[-1] ^ 1])
    _error("chunk-crc-mismatch", lambda: decode_png(_insert(encoded, 33, corrupt)))
    _error("chunk-before-ihdr", lambda: decode_png(_insert(encoded, 8, valid)))


def test_adler_vectors_and_reduction_boundary() -> None:
    assert adler32(b"Wikipedia") == 0x11E60398
    boundary = bytes(index & 0xFF for index in range(5553))
    assert adler32(boundary) == 0x2CCAB2EF
