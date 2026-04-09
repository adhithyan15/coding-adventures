"""Tests for coding-adventures-image-codec-qoi."""
import pytest
from image_codec_qoi import QoiCodec, decode_qoi, encode_qoi
from pixel_container import create_pixel_container, fill_pixels, pixel_at, set_pixel


# ============================================================================
# encode_qoi — header structure
# ============================================================================


def test_magic_bytes():
    c = create_pixel_container(1, 1)
    qoi = encode_qoi(c)
    assert qoi[:4] == b"qoif"


def test_dimensions_in_header():
    import struct
    c = create_pixel_container(7, 3)
    qoi = encode_qoi(c)
    w, h = struct.unpack_from(">II", qoi, 4)
    assert w == 7
    assert h == 3


def test_channels_and_colorspace():
    c = create_pixel_container(1, 1)
    qoi = encode_qoi(c)
    assert qoi[12] == 4   # channels = RGBA
    assert qoi[13] == 0   # colorspace = sRGB


def test_end_marker():
    c = create_pixel_container(1, 1)
    qoi = encode_qoi(c)
    assert qoi[-8:] == bytes([0, 0, 0, 0, 0, 0, 0, 1])


# ============================================================================
# encode_qoi — ops
# ============================================================================


def test_run_op_for_solid_image():
    """A 1x62 solid colour image should use exactly one OP_RUN byte.

    Initial prev=(0,0,0,255). First pixel (100,150,200,255) has matching
    alpha, so it encodes as OP_RGB (4 bytes). Pixels 2..62 all match prev,
    forming a run of 61 that emits one OP_RUN byte.
    """
    c = create_pixel_container(62, 1)
    fill_pixels(c, 100, 150, 200, 255)
    qoi = encode_qoi(c)
    # 14-byte header + first pixel (OP_RGB = 4 bytes) + 1 OP_RUN + 8-byte end
    assert len(qoi) == 14 + 4 + 1 + 8


def test_solid_image_compressed():
    """100×100 solid image should be much smaller than raw."""
    c = create_pixel_container(100, 100)
    fill_pixels(c, 200, 100, 50, 255)
    qoi = encode_qoi(c)
    assert len(qoi) < 250  # raw would be 40054 bytes


def test_index_op_reuses_hash_entry():
    """Two identical non-consecutive pixels should use OP_INDEX the second time."""
    c = create_pixel_container(3, 1)
    # pixel 0: unique colour that hashes to some slot
    set_pixel(c, 0, 0, 10, 20, 30, 255)
    # pixel 1: different colour
    set_pixel(c, 1, 0, 50, 60, 70, 255)
    # pixel 2: same as pixel 0 → should use OP_INDEX
    set_pixel(c, 2, 0, 10, 20, 30, 255)
    qoi = encode_qoi(c)
    # Must encode successfully and decode back correctly
    c2 = decode_qoi(qoi)
    assert pixel_at(c2, 2, 0) == (10, 20, 30, 255)


# ============================================================================
# decode_qoi — error cases
# ============================================================================


def test_decode_too_short():
    with pytest.raises(ValueError, match="too short"):
        decode_qoi(b"\x00" * 10)


def test_decode_bad_magic():
    with pytest.raises(ValueError, match="magic"):
        decode_qoi(b"XXXX" + b"\x00" * 18)


def test_decode_zero_dimensions():
    import struct
    data = bytearray(b"qoif" + struct.pack(">II", 0, 0) + bytes([4, 0]) + bytes(8))
    with pytest.raises(ValueError, match="dimensions"):
        decode_qoi(bytes(data))


def test_decode_truncated():
    import struct
    # Header claims 10x10 but payload is nearly empty
    hdr = b"qoif" + struct.pack(">II", 10, 10) + bytes([4, 0])
    with pytest.raises(ValueError):
        decode_qoi(hdr + bytes(8))  # just end marker, no pixel data


# ============================================================================
# encode → decode roundtrip
# ============================================================================


def test_roundtrip_1x1():
    c = create_pixel_container(1, 1)
    set_pixel(c, 0, 0, 100, 150, 200, 255)
    c2 = decode_qoi(encode_qoi(c))
    assert pixel_at(c2, 0, 0) == (100, 150, 200, 255)


def test_roundtrip_dimensions():
    c = create_pixel_container(5, 3)
    fill_pixels(c, 255, 0, 0, 255)
    c2 = decode_qoi(encode_qoi(c))
    assert c2.width == 5
    assert c2.height == 3


def test_roundtrip_with_alpha():
    c = create_pixel_container(2, 1)
    set_pixel(c, 0, 0, 255, 0, 0, 128)
    set_pixel(c, 1, 0, 0, 255, 0, 64)
    c2 = decode_qoi(encode_qoi(c))
    assert pixel_at(c2, 0, 0) == (255, 0, 0, 128)
    assert pixel_at(c2, 1, 0) == (0, 255, 0, 64)


def test_roundtrip_gradient():
    c = create_pixel_container(16, 16)
    for y in range(16):
        for x in range(16):
            set_pixel(c, x, y, x * 16, y * 16, 128, 255)
    c2 = decode_qoi(encode_qoi(c))
    for y in range(16):
        for x in range(16):
            assert pixel_at(c2, x, y) == (x * 16, y * 16, 128, 255)


def test_roundtrip_all_pixels():
    c = create_pixel_container(4, 4)
    for y in range(4):
        for x in range(4):
            set_pixel(c, x, y, x * 20, y * 20, 100, 200)
    c2 = decode_qoi(encode_qoi(c))
    for y in range(4):
        for x in range(4):
            assert pixel_at(c2, x, y) == (x * 20, y * 20, 100, 200)


def test_roundtrip_white_fill():
    c = create_pixel_container(8, 8)
    fill_pixels(c, 255, 255, 255, 255)
    c2 = decode_qoi(encode_qoi(c))
    for y in range(8):
        for x in range(8):
            assert pixel_at(c2, x, y) == (255, 255, 255, 255)


def test_roundtrip_small_diff():
    """Adjacent pixels with small RGB deltas should use OP_DIFF."""
    c = create_pixel_container(3, 1)
    set_pixel(c, 0, 0, 100, 100, 100, 255)
    set_pixel(c, 1, 0, 101, 99, 100, 255)   # dr=+1, dg=-1, db=0 — fits DIFF
    set_pixel(c, 2, 0, 102, 98, 100, 255)   # dr=+1, dg=-1, db=0 — fits DIFF
    c2 = decode_qoi(encode_qoi(c))
    assert pixel_at(c2, 1, 0) == (101, 99, 100, 255)
    assert pixel_at(c2, 2, 0) == (102, 98, 100, 255)


def test_roundtrip_luma():
    """Pixels within LUMA range encode/decode correctly."""
    c = create_pixel_container(2, 1)
    set_pixel(c, 0, 0, 100, 100, 100, 255)
    # dg=10, dr-dg=5, db-dg=-3 — fits LUMA range
    set_pixel(c, 1, 0, 115, 110, 107, 255)
    c2 = decode_qoi(encode_qoi(c))
    assert pixel_at(c2, 1, 0) == (115, 110, 107, 255)


# ============================================================================
# QoiCodec class
# ============================================================================


def test_codec_mime_type():
    assert QoiCodec().mime_type == "image/qoi"


def test_codec_encode_decode():
    codec = QoiCodec()
    c = create_pixel_container(2, 2)
    fill_pixels(c, 1, 2, 3, 4)
    c2 = codec.decode(codec.encode(c))
    assert pixel_at(c2, 0, 0) == (1, 2, 3, 4)
