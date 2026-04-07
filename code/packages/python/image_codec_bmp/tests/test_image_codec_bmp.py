"""Tests for coding-adventures-image-codec-bmp."""
import struct

import pytest
from image_codec_bmp import BmpCodec, decode_bmp, encode_bmp
from pixel_container import create_pixel_container, fill_pixels, pixel_at, set_pixel


# ============================================================================
# encode_bmp — header structure
# ============================================================================


def test_magic_bytes():
    c = create_pixel_container(1, 1)
    bmp = encode_bmp(c)
    assert bmp[:2] == b"BM"


def test_file_size():
    # 54-byte header + 1*1*4 = 58
    c = create_pixel_container(1, 1)
    bmp = encode_bmp(c)
    file_size = struct.unpack_from("<I", bmp, 2)[0]
    assert file_size == 58


def test_pixel_offset():
    c = create_pixel_container(2, 2)
    bmp = encode_bmp(c)
    offset = struct.unpack_from("<I", bmp, 10)[0]
    assert offset == 54


def test_info_header_size():
    c = create_pixel_container(1, 1)
    bmp = encode_bmp(c)
    bi_size = struct.unpack_from("<I", bmp, 14)[0]
    assert bi_size == 40


def test_width_height_in_header():
    c = create_pixel_container(7, 3)
    bmp = encode_bmp(c)
    w, h = struct.unpack_from("<ii", bmp, 18)
    assert w == 7
    assert h == -3  # negative = top-down


def test_bit_count_32():
    c = create_pixel_container(1, 1)
    bmp = encode_bmp(c)
    bit_count = struct.unpack_from("<H", bmp, 28)[0]
    assert bit_count == 32


def test_compression_zero():
    c = create_pixel_container(1, 1)
    bmp = encode_bmp(c)
    compression = struct.unpack_from("<I", bmp, 30)[0]
    assert compression == 0


# ============================================================================
# encode_bmp — pixel data (RGBA → BGRA swap)
# ============================================================================


def test_rgba_to_bgra_swap():
    """Encoder must write BGRA order in the file."""
    c = create_pixel_container(1, 1)
    set_pixel(c, 0, 0, 255, 128, 64, 200)  # R=255 G=128 B=64 A=200
    bmp = encode_bmp(c)
    b, g, r, a = bmp[54], bmp[55], bmp[56], bmp[57]
    assert (r, g, b, a) == (255, 128, 64, 200)


def test_black_pixel():
    c = create_pixel_container(1, 1)
    # default is all zeros
    bmp = encode_bmp(c)
    assert bmp[54:58] == bytes([0, 0, 0, 0])


def test_2x1_pixel_order():
    c = create_pixel_container(2, 1)
    set_pixel(c, 0, 0, 10, 20, 30, 40)
    set_pixel(c, 1, 0, 50, 60, 70, 80)
    bmp = encode_bmp(c)
    # pixel 0: BGRA = 30,20,10,40
    assert bmp[54:58] == bytes([30, 20, 10, 40])
    # pixel 1: BGRA = 70,60,50,80
    assert bmp[58:62] == bytes([70, 60, 50, 80])


# ============================================================================
# decode_bmp — error cases
# ============================================================================


def test_decode_too_short():
    with pytest.raises(ValueError, match="too short"):
        decode_bmp(b"\x00" * 10)


def test_decode_bad_magic():
    with pytest.raises(ValueError, match="magic"):
        decode_bmp(b"XX" + b"\x00" * 52)


def test_decode_unsupported_bit_depth():
    c = create_pixel_container(1, 1)
    bmp = bytearray(encode_bmp(c))
    struct.pack_into("<H", bmp, 28, 24)  # change to 24bpp
    with pytest.raises(ValueError, match="bit depth"):
        decode_bmp(bytes(bmp))


def test_decode_unsupported_compression():
    c = create_pixel_container(1, 1)
    bmp = bytearray(encode_bmp(c))
    struct.pack_into("<I", bmp, 30, 1)  # BI_RLE8
    with pytest.raises(ValueError, match="compression"):
        decode_bmp(bytes(bmp))


def test_decode_truncated_pixel_data():
    c = create_pixel_container(4, 4)
    bmp = encode_bmp(c)[:60]  # cut off most pixels
    with pytest.raises(ValueError, match="truncated"):
        decode_bmp(bmp)


# ============================================================================
# encode → decode roundtrip
# ============================================================================


def test_roundtrip_1x1():
    c = create_pixel_container(1, 1)
    set_pixel(c, 0, 0, 100, 150, 200, 255)
    c2 = decode_bmp(encode_bmp(c))
    assert pixel_at(c2, 0, 0) == (100, 150, 200, 255)


def test_roundtrip_dimensions():
    c = create_pixel_container(5, 3)
    fill_pixels(c, 255, 0, 0, 255)
    c2 = decode_bmp(encode_bmp(c))
    assert c2.width == 5
    assert c2.height == 3


def test_roundtrip_all_pixels():
    c = create_pixel_container(4, 4)
    for y in range(4):
        for x in range(4):
            set_pixel(c, x, y, x * 10, y * 10, 50, 255)
    c2 = decode_bmp(encode_bmp(c))
    for y in range(4):
        for x in range(4):
            assert pixel_at(c2, x, y) == (x * 10, y * 10, 50, 255)


def test_roundtrip_alpha():
    c = create_pixel_container(2, 2)
    set_pixel(c, 0, 0, 10, 20, 30, 0)
    set_pixel(c, 1, 0, 40, 50, 60, 127)
    set_pixel(c, 0, 1, 70, 80, 90, 200)
    set_pixel(c, 1, 1, 255, 255, 255, 255)
    c2 = decode_bmp(encode_bmp(c))
    assert pixel_at(c2, 0, 0) == (10, 20, 30, 0)
    assert pixel_at(c2, 1, 0) == (40, 50, 60, 127)
    assert pixel_at(c2, 0, 1) == (70, 80, 90, 200)
    assert pixel_at(c2, 1, 1) == (255, 255, 255, 255)


def test_roundtrip_white_fill():
    c = create_pixel_container(8, 8)
    fill_pixels(c, 255, 255, 255, 255)
    c2 = decode_bmp(encode_bmp(c))
    for y in range(8):
        for x in range(8):
            assert pixel_at(c2, x, y) == (255, 255, 255, 255)


# ============================================================================
# BmpCodec class
# ============================================================================


def test_codec_mime_type():
    assert BmpCodec().mime_type == "image/bmp"


def test_codec_encode_decode():
    codec = BmpCodec()
    c = create_pixel_container(2, 2)
    fill_pixels(c, 1, 2, 3, 4)
    c2 = codec.decode(codec.encode(c))
    assert pixel_at(c2, 0, 0) == (1, 2, 3, 4)


def test_decode_bottom_up():
    """Construct a bottom-up BMP (positive biHeight) and verify row order."""
    c = create_pixel_container(2, 2)
    set_pixel(c, 0, 0, 10, 0, 0, 255)  # row 0
    set_pixel(c, 0, 1, 20, 0, 0, 255)  # row 1
    bmp = bytearray(encode_bmp(c))
    # Flip biHeight to positive (bottom-up)
    struct.pack_into("<ii", bmp, 18, 2, 2)
    # Re-order pixels: bottom-up means last row first
    # Pixel at file row 0 = image row 1, pixel at file row 1 = image row 0
    # Current BGRA pixel at offset 54 (file row 0) should be for image row 1
    # Swap the two row pixel data blocks
    row0 = bytes(bmp[54:62])
    row1 = bytes(bmp[62:70])
    bmp[54:62] = row1
    bmp[62:70] = row0
    c2 = decode_bmp(bytes(bmp))
    assert pixel_at(c2, 0, 0) == (10, 0, 0, 255)
    assert pixel_at(c2, 0, 1) == (20, 0, 0, 255)
