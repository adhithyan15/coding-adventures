"""
coding-adventures-image-codec-bmp

IC01: BMP image encoder and decoder.

Produces 32-bit BGRA BMP files (biBitCount=32, biCompression=BI_RGB).
Negative biHeight → top-down layout (row 0 in file = top of image).
The decoder handles both top-down and bottom-up files.
"""
from __future__ import annotations

import struct

from pixel_container import ImageCodec, PixelContainer, create_pixel_container

__all__ = ["BmpCodec", "encode_bmp", "decode_bmp"]


class BmpCodec(ImageCodec):
    """BMP image encoder and decoder."""

    @property
    def mime_type(self) -> str:
        return "image/bmp"

    def encode(self, container: PixelContainer) -> bytes:
        return encode_bmp(container)

    def decode(self, data: bytes) -> PixelContainer:
        return decode_bmp(data)


def encode_bmp(pixels: PixelContainer) -> bytes:
    """Encode a PixelContainer to 32-bit BMP bytes.

    Example::

        c = create_pixel_container(2, 1)
        set_pixel(c, 0, 0, 255, 0, 0, 255)  # red
        bmp = encode_bmp(c)
        assert bmp[:2] == b"BM"
    """
    width, height = pixels.width, pixels.height
    pixel_bytes = width * height * 4
    file_size = 54 + pixel_bytes

    # BITMAPFILEHEADER (14 bytes)
    header = struct.pack("<2sIHHI", b"BM", file_size, 0, 0, 54)
    # BITMAPINFOHEADER (40 bytes)
    info = struct.pack(
        "<IiiHHIIiiII",
        40,          # biSize
        width,       # biWidth
        -height,     # biHeight (negative = top-down)
        1,           # biPlanes
        32,          # biBitCount
        0,           # biCompression (BI_RGB)
        pixel_bytes, # biSizeImage
        0, 0,        # biXPelsPerMeter, biYPelsPerMeter
        0, 0,        # biClrUsed, biClrImportant
    )

    # Pixel data: RGBA → BGRA
    buf = bytearray(pixel_bytes)
    data = pixels.data
    for i in range(0, len(data), 4):
        j = i
        buf[j]     = data[i + 2]  # B
        buf[j + 1] = data[i + 1]  # G
        buf[j + 2] = data[i]      # R
        buf[j + 3] = data[i + 3]  # A

    return header + info + bytes(buf)


def decode_bmp(data: bytes) -> PixelContainer:
    """Decode BMP bytes into a PixelContainer.

    Only 32-bit BGRA BI_RGB files are supported. Raises ``ValueError`` on
    invalid input.
    """
    if len(data) < 54:
        raise ValueError("BMP: file too short")
    if data[0:2] != b"BM":
        raise ValueError("BMP: invalid magic")

    pixel_offset = struct.unpack_from("<I", data, 10)[0]
    if pixel_offset < 54:
        raise ValueError("BMP: pixel offset is before end of header")

    bi_width, bi_height = struct.unpack_from("<ii", data, 18)
    if bi_width <= 0:
        raise ValueError("BMP: invalid width")
    if bi_height == -(2 ** 31):
        raise ValueError("BMP: invalid height")

    width = bi_width
    height = abs(bi_height)
    top_down = bi_height < 0
    if height == 0:
        raise ValueError("BMP: invalid height")

    bit_count = struct.unpack_from("<H", data, 28)[0]
    compression = struct.unpack_from("<I", data, 30)[0]
    if bit_count != 32:
        raise ValueError(f"BMP: unsupported bit depth {bit_count}, only 32 supported")
    if compression != 0:
        raise ValueError(f"BMP: unsupported compression {compression}")

    pixel_bytes = width * height * 4
    pixel_end = pixel_offset + pixel_bytes
    if len(data) < pixel_end:
        raise ValueError("BMP: pixel data truncated")

    container = create_pixel_container(width, height)
    for row in range(height):
        dest_row = row if top_down else height - 1 - row
        for col in range(width):
            file_idx = pixel_offset + (row * width + col) * 4
            b = data[file_idx]
            g = data[file_idx + 1]
            r = data[file_idx + 2]
            a = data[file_idx + 3]
            dest_idx = (dest_row * width + col) * 4
            container.data[dest_idx]     = r
            container.data[dest_idx + 1] = g
            container.data[dest_idx + 2] = b
            container.data[dest_idx + 3] = a

    return container
