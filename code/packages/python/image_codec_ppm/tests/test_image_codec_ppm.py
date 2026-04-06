"""Tests for coding-adventures-image-codec-ppm."""
import pytest
from image_codec_ppm import PpmCodec, decode_ppm, encode_ppm
from pixel_container import create_pixel_container, fill_pixels, pixel_at, set_pixel


# ============================================================================
# encode_ppm — header
# ============================================================================


def test_magic_header():
    c = create_pixel_container(1, 1)
    ppm = encode_ppm(c)
    assert ppm.startswith(b"P6\n")


def test_dimensions_in_header():
    c = create_pixel_container(7, 3)
    ppm = encode_ppm(c)
    header_end = ppm.index(b"\n", ppm.index(b"\n") + 1)
    first_line = ppm[:ppm.index(b"\n") + 1]
    assert b"7 3" in ppm[:40]


def test_maxval_in_header():
    c = create_pixel_container(2, 2)
    ppm = encode_ppm(c)
    assert b"255" in ppm[:20]


def test_pixel_data_size():
    c = create_pixel_container(3, 2)
    ppm = encode_ppm(c)
    header = b"P6\n3 2\n255\n"
    assert len(ppm) == len(header) + 3 * 2 * 3


# ============================================================================
# encode_ppm — alpha dropped
# ============================================================================


def test_alpha_dropped():
    """PPM has no alpha channel — pixel bytes must be RGB only."""
    c = create_pixel_container(1, 1)
    set_pixel(c, 0, 0, 100, 150, 200, 127)
    ppm = encode_ppm(c)
    header = b"P6\n1 1\n255\n"
    pixels = ppm[len(header):]
    assert len(pixels) == 3
    assert pixels == bytes([100, 150, 200])


def test_2x1_rgb_order():
    c = create_pixel_container(2, 1)
    set_pixel(c, 0, 0, 10, 20, 30, 255)
    set_pixel(c, 1, 0, 40, 50, 60, 255)
    ppm = encode_ppm(c)
    header = b"P6\n2 1\n255\n"
    pixels = ppm[len(header):]
    assert pixels == bytes([10, 20, 30, 40, 50, 60])


# ============================================================================
# decode_ppm — error cases
# ============================================================================


def test_decode_bad_magic():
    with pytest.raises(ValueError, match="P6"):
        decode_ppm(b"P3\n1 1\n255\n" + bytes([0] * 3))


def test_decode_unsupported_maxval():
    with pytest.raises(ValueError, match="255"):
        decode_ppm(b"P6\n1 1\n256\n" + bytes([0] * 3))


def test_decode_truncated():
    with pytest.raises(ValueError, match="truncated"):
        decode_ppm(b"P6\n2 2\n255\n" + bytes([0] * 5))  # need 12 bytes


def test_decode_bad_dimensions_text():
    with pytest.raises((ValueError, Exception)):
        decode_ppm(b"P6\nabc def\n255\n" + bytes([0] * 3))


# ============================================================================
# decode_ppm — alpha set to 255
# ============================================================================


def test_decoded_alpha_is_255():
    c = create_pixel_container(1, 1)
    set_pixel(c, 0, 0, 10, 20, 30, 0)
    ppm = encode_ppm(c)
    c2 = decode_ppm(ppm)
    assert pixel_at(c2, 0, 0)[3] == 255


# ============================================================================
# encode → decode roundtrip
# ============================================================================


def test_roundtrip_1x1():
    c = create_pixel_container(1, 1)
    set_pixel(c, 0, 0, 100, 150, 200, 255)
    c2 = decode_ppm(encode_ppm(c))
    r, g, b, a = pixel_at(c2, 0, 0)
    assert (r, g, b) == (100, 150, 200)
    assert a == 255


def test_roundtrip_dimensions():
    c = create_pixel_container(5, 3)
    c2 = decode_ppm(encode_ppm(c))
    assert c2.width == 5
    assert c2.height == 3


def test_roundtrip_all_pixels():
    c = create_pixel_container(4, 4)
    for y in range(4):
        for x in range(4):
            set_pixel(c, x, y, x * 10, y * 10, 50, 255)
    c2 = decode_ppm(encode_ppm(c))
    for y in range(4):
        for x in range(4):
            r, g, b, _ = pixel_at(c2, x, y)
            assert (r, g, b) == (x * 10, y * 10, 50)


def test_roundtrip_white_fill():
    c = create_pixel_container(8, 8)
    fill_pixels(c, 255, 255, 255, 255)
    c2 = decode_ppm(encode_ppm(c))
    for y in range(8):
        for x in range(8):
            r, g, b, _ = pixel_at(c2, x, y)
            assert (r, g, b) == (255, 255, 255)


def test_roundtrip_black_fill():
    c = create_pixel_container(4, 4)
    fill_pixels(c, 0, 0, 0, 255)
    c2 = decode_ppm(encode_ppm(c))
    for y in range(4):
        for x in range(4):
            r, g, b, _ = pixel_at(c2, x, y)
            assert (r, g, b) == (0, 0, 0)


def test_roundtrip_comments_skipped():
    """Decoder must skip comment lines in PPM header."""
    ppm = b"P6\n# a comment\n2 1\n255\n" + bytes([10, 20, 30, 40, 50, 60])
    c = decode_ppm(ppm)
    assert c.width == 2
    assert c.height == 1
    assert pixel_at(c, 0, 0)[:3] == (10, 20, 30)
    assert pixel_at(c, 1, 0)[:3] == (40, 50, 60)


# ============================================================================
# PpmCodec class
# ============================================================================


def test_codec_mime_type():
    assert PpmCodec().mime_type == "image/x-portable-pixmap"


def test_codec_encode_decode():
    codec = PpmCodec()
    c = create_pixel_container(2, 2)
    fill_pixels(c, 1, 2, 3, 255)
    c2 = codec.decode(codec.encode(c))
    r, g, b, _ = pixel_at(c2, 0, 0)
    assert (r, g, b) == (1, 2, 3)
