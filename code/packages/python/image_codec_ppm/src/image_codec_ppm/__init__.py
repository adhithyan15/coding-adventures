"""
coding-adventures-image-codec-ppm

IC02: PPM P6 image encoder and decoder.

File format: ``P6\\n<width> <height>\\n255\\n<RGB bytes>``.
No alpha channel — dropped on encode, set to 255 on decode.
"""
from __future__ import annotations

from pixel_container import ImageCodec, PixelContainer, create_pixel_container

__all__ = ["PpmCodec", "encode_ppm", "decode_ppm"]

_MAX_DIMENSION = 16384
_MAX_TOKEN_LEN = 20


class PpmCodec(ImageCodec):
    """PPM P6 image encoder and decoder."""

    @property
    def mime_type(self) -> str:
        return "image/x-portable-pixmap"

    def encode(self, container: PixelContainer) -> bytes:
        return encode_ppm(container)

    def decode(self, data: bytes) -> PixelContainer:
        return decode_ppm(data)


def encode_ppm(pixels: PixelContainer) -> bytes:
    """Encode a PixelContainer to PPM P6 bytes.

    Alpha is dropped (PPM has no alpha channel).

    Example::

        c = create_pixel_container(2, 1)
        set_pixel(c, 0, 0, 255, 0, 0, 255)
        ppm = encode_ppm(c)
        assert ppm.startswith(b"P6\\n")
    """
    w, h = pixels.width, pixels.height
    header = f"P6\n{w} {h}\n255\n".encode()
    buf = bytearray(len(header) + w * h * 3)
    buf[:len(header)] = header
    off = len(header)
    data = pixels.data
    for i in range(0, len(data), 4):
        buf[off]     = data[i]      # R
        buf[off + 1] = data[i + 1]  # G
        buf[off + 2] = data[i + 2]  # B
        off += 3
    return bytes(buf)


def decode_ppm(data: bytes) -> PixelContainer:
    """Decode PPM P6 bytes into a PixelContainer.

    Decoded pixels have A=255. Raises ``ValueError`` on invalid input.
    """
    pos = 0

    def _skip() -> None:
        nonlocal pos
        while True:
            # skip whitespace
            while pos < len(data) and data[pos:pos+1] in (b" ", b"\t", b"\r", b"\n"):
                pos += 1
            # skip comments
            if pos < len(data) and data[pos:pos+1] == b"#":
                while pos < len(data) and data[pos:pos+1] != b"\n":
                    pos += 1
            else:
                break

    def _read_token() -> str:
        nonlocal pos
        _skip()
        if pos >= len(data):
            raise ValueError("PPM: unexpected end of header")
        start = pos
        while pos < len(data) and data[pos:pos+1] not in (b" ", b"\t", b"\r", b"\n"):
            pos += 1
            if pos - start > _MAX_TOKEN_LEN:
                raise ValueError("PPM: header token too long")
        return data[start:pos].decode()

    magic = _read_token()
    if magic != "P6":
        raise ValueError("PPM: invalid magic, expected P6")

    width = int(_read_token())
    height = int(_read_token())
    if width <= 0 or height <= 0:
        raise ValueError(f"PPM: invalid dimensions ({width}×{height})")
    if width > _MAX_DIMENSION or height > _MAX_DIMENSION:
        raise ValueError(f"PPM: dimensions {width}×{height} exceed maximum {_MAX_DIMENSION}")
    maxval = int(_read_token())
    if maxval != 255:
        raise ValueError(f"PPM: unsupported max value {maxval}, only 255 supported")

    # Skip exactly one whitespace byte after maxval.
    if pos >= len(data):
        raise ValueError("PPM: pixel data truncated")
    pos += 1

    needed = width * height * 3
    if len(data) - pos < needed:
        raise ValueError("PPM: pixel data truncated")

    container = create_pixel_container(width, height)
    for p in range(width * height):
        r = data[pos]
        g = data[pos + 1]
        b = data[pos + 2]
        pos += 3
        base = p * 4
        container.data[base]     = r
        container.data[base + 1] = g
        container.data[base + 2] = b
        container.data[base + 3] = 255

    return container
