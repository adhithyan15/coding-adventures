from __future__ import annotations

import io
import json
import zlib
from collections.abc import Callable
from pathlib import Path
from typing import Any, cast

import pytest
from PIL import Image
from pixel_container import PixelContainer  # type: ignore[import-untyped]

from image_codec_png import (
    PNG_ERROR_CODES,
    PNG_MAX_DIMENSION,
    PNG_MAX_PIXELS,
    PngError,
    adler32,
    decode_png,
    encode_png,
)

FIXTURE_PATH = (
    Path(__file__).parents[4] / "specs" / "fixtures" / "image-codec-png-v1" / "cases.json"
)
FIXTURES = cast(dict[str, Any], json.loads(FIXTURE_PATH.read_text(encoding="utf-8")))
CASES = cast(list[dict[str, Any]], FIXTURES["cases"])


def _pixels(value: dict[str, Any]) -> PixelContainer:
    """Construct fixture state without PixelContainer's allocation side effect."""
    result = object.__new__(PixelContainer)
    result.width = value["width"]
    result.height = value["height"]
    result.data = bytearray.fromhex(cast(str, value["rgba_hex"]))
    return result


def _chunks(png: bytes) -> list[tuple[str, bytes]]:
    chunks: list[tuple[str, bytes]] = []
    position = 8
    while position < len(png):
        length = int.from_bytes(png[position : position + 4], "big")
        chunk_type = png[position + 4 : position + 8].decode("ascii")
        chunks.append((chunk_type, png[position + 8 : position + 8 + length]))
        position += 12 + length
    return chunks


def _expect_error(expected: str, action: Callable[[], object]) -> None:
    with pytest.raises(PngError) as raised:
        action()
    assert raised.value.code == expected
    assert str(raised.value) == expected


def test_pins_public_contract() -> None:
    limits = cast(dict[str, int], FIXTURES["limits"])
    assert limits["max_dimension"] == PNG_MAX_DIMENSION
    assert limits["default_max_pixels"] == PNG_MAX_PIXELS
    assert list(PNG_ERROR_CODES) == FIXTURES["error_ids"]
    assert len(CASES) == 85


@pytest.mark.parametrize("case", CASES, ids=[cast(str, case["id"]) for case in CASES])
def test_portable_case(case: dict[str, Any]) -> None:
    operation = cast(str, case["operation"])
    expected = cast(dict[str, Any], case["expected"])
    options = cast(dict[str, Any] | None, case.get("options"))
    max_pixels = options["max_pixels"] if options is not None else None

    if operation == "decode":
        actual = decode_png(bytes.fromhex(cast(str, case["png_hex"])), max_pixels=max_pixels)
        assert actual.width == expected["width"]
        assert actual.height == expected["height"]
        assert bytes(actual.data) == bytes.fromhex(cast(str, expected["rgba_hex"]))
        return

    if operation == "decode-error":
        _expect_error(
            cast(str, expected["error_id"]),
            lambda: decode_png(bytes.fromhex(cast(str, case["png_hex"])), max_pixels=max_pixels),
        )
        return

    if operation == "encode":
        input_pixels = cast(dict[str, Any], case["input"])
        encoded = encode_png(_pixels(input_pixels))
        chunks = _chunks(encoded)
        assert [chunk_type for chunk_type, _ in chunks] == expected["chunk_types"]
        assert encoded[24] == expected["bit_depth"]
        assert encoded[25] == expected["colour_type"]
        assert encoded[28] == expected["interlace"]
        idat = b"".join(payload for chunk_type, payload in chunks if chunk_type == "IDAT")
        filtered = zlib.decompress(idat)
        stride = cast(int, input_pixels["width"]) * 4
        assert [
            filtered[row * (stride + 1)] for row in range(cast(int, input_pixels["height"]))
        ] == expected["filter_types"]
        with Image.open(io.BytesIO(encoded)) as image:
            foreign = image.convert("RGBA")
            assert foreign.size == (input_pixels["width"], input_pixels["height"])
            assert foreign.tobytes() == bytes.fromhex(cast(str, input_pixels["rgba_hex"]))
        return

    if operation == "encode-error":
        _expect_error(
            cast(str, expected["error_id"]),
            lambda: encode_png(_pixels(cast(dict[str, Any], case["input"]))),
        )
        return

    assert operation == "adler32"
    assert f"{adler32(bytes.fromhex(cast(str, case['input_hex']))):08x}" == expected["adler32_hex"]
