"""IC18: bounded PNG encoding and decoding over repository primitives."""

from __future__ import annotations

import struct
from typing import NoReturn

from coding_adventures_zip import (  # type: ignore[import-untyped]
    RawInflateError,
    crc32,
    raw_deflate,
    raw_inflate_counted,
)
from pixel_container import ImageCodec, PixelContainer  # type: ignore[import-untyped]

__all__ = [
    "PNG_ERROR_CODES",
    "PNG_MAX_DIMENSION",
    "PNG_MAX_PIXELS",
    "PngCodec",
    "PngError",
    "adler32",
    "codec",
    "decode_png",
    "encode_png",
]

PNG_MAX_DIMENSION = 16_384
PNG_MAX_PIXELS = 32 * 1024 * 1024

PNG_ERROR_CODES = (
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


class PngError(ValueError):
    """Stable, payload-blind IC18 failure."""

    code: str

    def __init__(self, code: str) -> None:
        if code not in PNG_ERROR_CODES:
            raise ValueError("unknown PNG error code")
        self.code = code
        super().__init__(code)


def _fail(code: str) -> NoReturn:
    raise PngError(code)


def _validate_max_pixels(requested: object | None) -> int:
    if requested is None:
        return PNG_MAX_PIXELS
    if type(requested) is not int or not 0 < requested <= PNG_MAX_PIXELS:
        _fail("invalid-max-pixels")
    return requested


class PngCodec(ImageCodec):  # type: ignore[misc]
    """ImageCodec adapter with an eagerly validated pixel ceiling."""

    _max_pixels: int

    def __init__(self, *, max_pixels: object | None = None) -> None:
        self._max_pixels = _validate_max_pixels(max_pixels)

    @property
    def mime_type(self) -> str:
        return "image/png"

    def encode(self, container: PixelContainer) -> bytes:
        return encode_png(container)

    def decode(self, data: bytes) -> PixelContainer:
        return _decode_png_with_limit(data, self._max_pixels)


_SIGNATURE = b"\x89PNG\r\n\x1a\n"
_ADLER_MOD = 65_521


def adler32(data: bytes | bytearray | memoryview) -> int:
    """Return the RFC 1950 Adler-32 checksum of *data*."""

    view = memoryview(data).cast("B")
    a = 1
    b = 0
    for start in range(0, len(view), 5552):
        for value in view[start : start + 5552]:
            a += value
            b += a
        a %= _ADLER_MOD
        b %= _ADLER_MOD
    return ((b << 16) | a) & 0xFFFF_FFFF


def _paeth(a: int, b: int, c: int) -> int:
    prediction = a + b - c
    distance_a = abs(prediction - a)
    distance_b = abs(prediction - b)
    distance_c = abs(prediction - c)
    if distance_a <= distance_b and distance_a <= distance_c:
        return a
    if distance_b <= distance_c:
        return b
    return c


def _apply_filter(
    filter_id: int,
    raw: memoryview,
    prior: bytearray,
    bytes_per_pixel: int,
    output: bytearray,
) -> None:
    for index, value in enumerate(raw):
        left = raw[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
        above = prior[index]
        above_left = prior[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
        if filter_id == 1:
            predicted = left
        elif filter_id == 2:
            predicted = above
        elif filter_id == 3:
            predicted = (left + above) // 2
        elif filter_id == 4:
            predicted = _paeth(left, above, above_left)
        else:
            predicted = 0
        output[index] = (value - predicted) & 0xFF


def _choose_filter(
    raw: memoryview,
    prior: bytearray,
    bytes_per_pixel: int,
    scratch: bytearray,
    best: bytearray,
) -> int:
    best_filter = 0
    best_score: int | None = None
    for filter_id in range(5):
        _apply_filter(filter_id, raw, prior, bytes_per_pixel, scratch)
        score = sum(value if value < 128 else 256 - value for value in scratch)
        if best_score is None or score < best_score:
            best_score = score
            best_filter = filter_id
            best[:] = scratch
    return best_filter


def _undo_filter(filter_id: int, row: bytearray, prior: bytearray, bytes_per_pixel: int) -> None:
    if filter_id == 0:
        return
    if filter_id == 1:
        for index in range(bytes_per_pixel, len(row)):
            row[index] = (row[index] + row[index - bytes_per_pixel]) & 0xFF
        return
    if filter_id == 2:
        for index in range(len(row)):
            row[index] = (row[index] + prior[index]) & 0xFF
        return
    if filter_id == 3:
        for index in range(len(row)):
            left = row[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
            row[index] = (row[index] + (left + prior[index]) // 2) & 0xFF
        return
    if filter_id == 4:
        for index in range(len(row)):
            left = row[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
            above_left = prior[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
            row[index] = (row[index] + _paeth(left, prior[index], above_left)) & 0xFF
        return
    _fail("invalid-filter")


def _chunk_parts(chunk_type: bytes, *payload_parts: bytes) -> tuple[bytes, ...]:
    payload_length = sum(len(part) for part in payload_parts)
    checksum = crc32(chunk_type)
    for part in payload_parts:
        checksum = crc32(part, initial=checksum)
    return (
        struct.pack(">I", payload_length),
        chunk_type,
        *payload_parts,
        struct.pack(">I", checksum),
    )


def encode_png(pixels: PixelContainer) -> bytes:
    """Encode a valid RGBA8 PixelContainer as a deterministic portable PNG."""

    width = pixels.width
    height = pixels.height
    if (
        type(width) is not int
        or type(height) is not int
        or width <= 0
        or height <= 0
        or width > PNG_MAX_DIMENSION
        or height > PNG_MAX_DIMENSION
    ):
        _fail("invalid-image-dimensions")
    pixel_count = width * height
    if pixel_count > PNG_MAX_PIXELS:
        _fail("invalid-image-dimensions")
    if len(pixels.data) != pixel_count * 4:
        _fail("invalid-pixel-data-length")

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    stride = width * 4
    filtered = bytearray(height * (stride + 1))
    prior = bytearray(stride)
    scratch = bytearray(stride)
    best = bytearray(stride)
    pixel_view = memoryview(pixels.data).cast("B")
    for row_index in range(height):
        raw = pixel_view[row_index * stride : (row_index + 1) * stride]
        destination = row_index * (stride + 1)
        filtered[destination] = _choose_filter(raw, prior, 4, scratch, best)
        filtered[destination + 1 : destination + 1 + stride] = best
        prior[:] = raw

    adler = struct.pack(">I", adler32(filtered))
    deflated = raw_deflate(filtered)
    del filtered
    return b"".join(
        (
            _SIGNATURE,
            *_chunk_parts(b"IHDR", ihdr),
            *_chunk_parts(b"IDAT", b"\x78\x9c", deflated, adler),
            *_chunk_parts(b"IEND", b""),
        )
    )


def _valid_chunk_type(chunk_type: bytes) -> bool:
    if len(chunk_type) != 4 or chunk_type[2] & 0x20:
        return False
    return all(0x41 <= value <= 0x5A or 0x61 <= value <= 0x7A for value in chunk_type)


def decode_png(
    data: bytes | bytearray | memoryview, *, max_pixels: object | None = None
) -> PixelContainer:
    """Decode the bounded, non-interlaced, 8-bit IC18 PNG profile."""

    return _decode_png_with_limit(data, _validate_max_pixels(max_pixels))


def _decode_png_with_limit(data: bytes | bytearray | memoryview, limit: int) -> PixelContainer:
    raw = memoryview(data).cast("B")
    if len(raw) < len(_SIGNATURE):
        _fail("file-too-short")
    if raw[: len(_SIGNATURE)] != _SIGNATURE:
        _fail("invalid-signature")

    width = 0
    height = 0
    colour_type = 0
    saw_ihdr = False
    saw_iend = False
    saw_plte = False
    saw_trns = False
    in_idat = False
    idat_ended = False
    transparent_grey: int | None = None
    transparent_rgb: tuple[int, int, int] | None = None
    idat_parts: list[bytes] = []

    position = len(_SIGNATURE)
    while position < len(raw):
        if len(raw) - position < 8:
            _fail("truncated-chunk")
        length = int.from_bytes(raw[position : position + 4], "big")
        if length > len(raw) - position - 12:
            _fail("truncated-chunk")
        type_start = position + 4
        data_start = position + 8
        data_end = data_start + length
        chunk_type = bytes(raw[type_start:data_start])
        if not _valid_chunk_type(chunk_type):
            _fail("invalid-chunk-type")
        payload = bytes(raw[data_start:data_end])
        declared_crc = int.from_bytes(raw[data_end : data_end + 4], "big")
        actual_crc = crc32(chunk_type)
        actual_crc = crc32(payload, initial=actual_crc)
        if actual_crc != declared_crc:
            _fail("chunk-crc-mismatch")
        type_name = chunk_type.decode("ascii")
        if not saw_ihdr and type_name != "IHDR":
            _fail("chunk-before-ihdr")

        if type_name == "IHDR":
            if saw_ihdr:
                _fail("duplicate-ihdr")
            if length != 13:
                _fail("invalid-ihdr-length")
            width, height, bit_depth, colour_type, compression, filter_method, interlace = (
                struct.unpack(">IIBBBBB", payload)
            )
            if width == 0 or height == 0:
                _fail("invalid-dimensions")
            if width > PNG_MAX_DIMENSION or height > PNG_MAX_DIMENSION:
                _fail("dimension-limit")
            if width * height > limit:
                _fail("pixel-limit")
            if compression != 0 or filter_method != 0 or interlace != 0:
                _fail("unsupported-feature")
            if bit_depth != 8 or colour_type not in (0, 2, 4, 6):
                _fail("unsupported-feature")
            saw_ihdr = True
        elif type_name == "PLTE":
            if (
                saw_plte
                or idat_parts
                or saw_trns
                or colour_type not in (2, 6)
                or length < 3
                or length > 768
                or length % 3 != 0
            ):
                _fail("invalid-plte")
            saw_plte = True
        elif type_name == "tRNS":
            if saw_trns or idat_parts:
                _fail("invalid-trns")
            if colour_type == 0:
                if length != 2:
                    _fail("invalid-trns")
                transparent_grey = int.from_bytes(payload, "big")
                if transparent_grey > 255:
                    _fail("invalid-trns")
            elif colour_type == 2:
                if length != 6:
                    _fail("invalid-trns")
                samples = struct.unpack(">HHH", payload)
                if any(sample > 255 for sample in samples):
                    _fail("invalid-trns")
                transparent_rgb = samples
            else:
                _fail("invalid-trns")
            saw_trns = True
        elif type_name == "IDAT":
            if idat_ended:
                _fail("nonconsecutive-idat")
            idat_parts.append(payload)
            in_idat = True
        elif type_name == "IEND":
            if length != 0:
                _fail("invalid-iend")
            if data_end + 4 != len(raw):
                _fail("trailing-data")
            saw_iend = True
            position = data_end + 4
            continue
        elif type_name in ("acTL", "fcTL", "fdAT"):
            _fail("unsupported-feature")
        elif chunk_type[0] & 0x20 == 0:
            _fail("unknown-critical-chunk")

        if type_name != "IDAT" and in_idat:
            in_idat = False
            idat_ended = True
        position = data_end + 4

    if not saw_ihdr or not saw_iend or not idat_parts:
        _fail("missing-required-chunk")

    zlib_data = b"".join(idat_parts)
    if len(zlib_data) < 6:
        _fail("invalid-zlib-header")
    cmf = zlib_data[0]
    flg = zlib_data[1]
    if (cmf & 0x0F) != 8 or (cmf >> 4) > 7 or ((cmf << 8) | flg) % 31 != 0:
        _fail("invalid-zlib-header")
    if flg & 0x20:
        _fail("preset-dictionary")

    channels = {0: 1, 2: 3, 4: 2, 6: 4}[colour_type]
    stride = width * channels
    expected_length = height * (stride + 1)
    deflate_data = zlib_data[2:-4]
    try:
        inflated = raw_inflate_counted(deflate_data, max_output=expected_length)
    except RawInflateError as error:
        if error.code == "output-limit-exceeded":
            _fail("inflated-length-mismatch")
        _fail("inflate-failed")
    if len(inflated.output) != expected_length:
        _fail("inflated-length-mismatch")
    if inflated.bytes_consumed != len(deflate_data):
        _fail("idat-cavity")
    if adler32(inflated.output) != int.from_bytes(zlib_data[-4:], "big"):
        _fail("adler-mismatch")

    row_size = stride + 1
    for row_index in range(height):
        if inflated.output[row_index * row_size] > 4:
            _fail("invalid-filter")

    result = PixelContainer(width, height, bytearray(width * height * 4))
    prior = bytearray(stride)
    for row_index in range(height):
        source_offset = row_index * row_size
        filter_id = inflated.output[source_offset]
        row = bytearray(inflated.output[source_offset + 1 : source_offset + row_size])
        _undo_filter(filter_id, row, prior, channels)
        destination_row = row_index * width * 4
        for column in range(width):
            source = column * channels
            destination = destination_row + column * 4
            if channels == 1:
                value = row[source]
                result.data[destination : destination + 4] = bytes(
                    (value, value, value, 0 if transparent_grey == value else 255)
                )
            elif channels == 2:
                value = row[source]
                result.data[destination : destination + 4] = bytes(
                    (value, value, value, row[source + 1])
                )
            elif channels == 3:
                red, green, blue = row[source : source + 3]
                transparent = transparent_rgb == (red, green, blue)
                result.data[destination : destination + 4] = bytes(
                    (red, green, blue, 0 if transparent else 255)
                )
            else:
                result.data[destination : destination + 4] = row[source : source + 4]
        prior[:] = row
    return result


codec = PngCodec()
