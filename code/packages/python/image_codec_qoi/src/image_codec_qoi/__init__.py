"""
coding-adventures-image-codec-qoi

IC03: QOI (Quite OK Image) encoder and decoder.

Six operations: OP_RGB (0xFE), OP_RGBA (0xFF), OP_INDEX (00xxxxxx),
OP_DIFF (01rrggbb), OP_LUMA (10gggggg + next byte), OP_RUN (11rrrrrr).
Hash: (r*3 + g*5 + b*7 + a*11) % 64.
"""
from __future__ import annotations

import struct

from pixel_container import ImageCodec, PixelContainer, create_pixel_container

__all__ = ["QoiCodec", "encode_qoi", "decode_qoi"]

_MAX_DIMENSION = 16384

_MAGIC = b"qoif"
_END_MARKER = bytes([0, 0, 0, 0, 0, 0, 0, 1])

_OP_RGB  = 0xFE
_OP_RGBA = 0xFF
_TAG_INDEX = 0b00
_TAG_DIFF  = 0b01
_TAG_LUMA  = 0b10


def _hash(r: int, g: int, b: int, a: int) -> int:
    return (r * 3 + g * 5 + b * 7 + a * 11) % 64


def _wrap(delta: int) -> int:
    """Interpret an int as a signed 8-bit value in [-128, 127]."""
    d = ((delta & 0xFF) + 128) & 0xFF
    return d - 128


class QoiCodec(ImageCodec):
    """QOI image encoder and decoder."""

    @property
    def mime_type(self) -> str:
        return "image/qoi"

    def encode(self, container: PixelContainer) -> bytes:
        return encode_qoi(container)

    def decode(self, data: bytes) -> PixelContainer:
        return decode_qoi(data)


def encode_qoi(pixels: PixelContainer) -> bytes:
    """Encode a PixelContainer to QOI bytes.

    Example::

        c = create_pixel_container(4, 4)
        fill_pixels(c, 100, 150, 200, 255)
        qoi = encode_qoi(c)
        assert qoi[:4] == b"qoif"
    """
    width, height = pixels.width, pixels.height
    out = bytearray()
    out += _MAGIC
    out += struct.pack(">II", width, height)
    out += bytes([4, 0])  # channels=RGBA, colorspace=sRGB

    hash_table: list[tuple[int, int, int, int]] = [(0, 0, 0, 0)] * 64
    prev = (0, 0, 0, 255)
    run = 0
    total = width * height
    data = pixels.data

    for p in range(total):
        base = p * 4
        r, g, b, a = data[base], data[base + 1], data[base + 2], data[base + 3]

        if (r, g, b, a) == prev:
            run += 1
            if run == 62 or p == total - 1:
                out.append(0xC0 | (run - 1))
                run = 0
            continue

        if run > 0:
            out.append(0xC0 | (run - 1))
            run = 0

        h = _hash(r, g, b, a)
        if hash_table[h] == (r, g, b, a):
            out.append(_TAG_INDEX << 6 | h)
        else:
            hash_table[h] = (r, g, b, a)
            pr, pg, pb, pa = prev
            if a == pa:
                dr = _wrap(r - pr)
                dg = _wrap(g - pg)
                db = _wrap(b - pb)
                if -2 <= dr <= 1 and -2 <= dg <= 1 and -2 <= db <= 1:
                    out.append((_TAG_DIFF << 6) | ((dr + 2) << 4) | ((dg + 2) << 2) | (db + 2))
                else:
                    drdg = dr - dg
                    dbdg = db - dg
                    if -32 <= dg <= 31 and -8 <= drdg <= 7 and -8 <= dbdg <= 7:
                        out.append((_TAG_LUMA << 6) | (dg + 32))
                        out.append(((drdg + 8) << 4) | (dbdg + 8))
                    else:
                        out += bytes([_OP_RGB, r, g, b])
            else:
                out += bytes([_OP_RGBA, r, g, b, a])

        prev = (r, g, b, a)

    out += _END_MARKER
    return bytes(out)


def decode_qoi(data: bytes) -> PixelContainer:
    """Decode QOI bytes into a PixelContainer.

    Raises ``ValueError`` on invalid input.
    """
    if len(data) < 22:
        raise ValueError("QOI: file too short")
    if data[:4] != _MAGIC:
        raise ValueError("QOI: invalid magic")

    width, height = struct.unpack_from(">II", data, 4)
    if width == 0 or height == 0:
        raise ValueError("QOI: invalid dimensions")
    if width > _MAX_DIMENSION or height > _MAX_DIMENSION:
        raise ValueError(f"QOI: dimensions {width}×{height} exceed maximum {_MAX_DIMENSION}")

    total = width * height
    payload_len = len(data) - 22
    if total > payload_len * 62:
        raise ValueError("QOI: pixel data truncated")

    container = create_pixel_container(width, height)
    hash_table: list[tuple[int, int, int, int]] = [(0, 0, 0, 0)] * 64
    prev = (0, 0, 0, 255)

    pos = 14
    written = 0

    while written < total:
        if pos >= len(data):
            raise ValueError("QOI: unexpected end of data")
        tag = data[pos]
        pos += 1

        if tag == _OP_RGB:
            if pos + 3 > len(data):
                raise ValueError("QOI: unexpected end of data")
            r, g, b, a = data[pos], data[pos + 1], data[pos + 2], prev[3]
            pos += 3
        elif tag == _OP_RGBA:
            if pos + 4 > len(data):
                raise ValueError("QOI: unexpected end of data")
            r, g, b, a = data[pos], data[pos + 1], data[pos + 2], data[pos + 3]
            pos += 4
        else:
            tag_bits = tag >> 6
            if tag_bits == _TAG_INDEX:
                r, g, b, a = hash_table[tag & 0x3F]
                base = written * 4
                container.data[base:base + 4] = bytes([r, g, b, a])
                written += 1
                prev = (r, g, b, a)
                continue
            elif tag_bits == _TAG_DIFF:
                dr = ((tag >> 4) & 0x3) - 2
                dg = ((tag >> 2) & 0x3) - 2
                db = ((tag >> 0) & 0x3) - 2
                r = (prev[0] + dr) & 0xFF
                g = (prev[1] + dg) & 0xFF
                b = (prev[2] + db) & 0xFF
                a = prev[3]
            elif tag_bits == _TAG_LUMA:
                if pos >= len(data):
                    raise ValueError("QOI: unexpected end of data")
                nxt = data[pos]
                pos += 1
                dg = (tag & 0x3F) - 32
                drdg = ((nxt >> 4) & 0xF) - 8
                dbdg = ((nxt >> 0) & 0xF) - 8
                r = (prev[0] + drdg + dg) & 0xFF
                g = (prev[1] + dg) & 0xFF
                b = (prev[2] + dbdg + dg) & 0xFF
                a = prev[3]
            else:  # OP_RUN
                run_len = (tag & 0x3F) + 1
                actual = min(run_len, total - written)
                r, g, b, a = prev
                for _ in range(actual):
                    base = written * 4
                    container.data[base:base + 4] = bytes([r, g, b, a])
                    written += 1
                continue

        hash_table[_hash(r, g, b, a)] = (r, g, b, a)
        base = written * 4
        container.data[base:base + 4] = bytes([r, g, b, a])
        written += 1
        prev = (r, g, b, a)

    return container
