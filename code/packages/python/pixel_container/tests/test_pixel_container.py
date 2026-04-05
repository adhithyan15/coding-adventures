"""Tests for coding-adventures-pixel-container."""
import pytest
from pixel_container import (
    PixelContainer,
    ImageCodec,
    create_pixel_container,
    pixel_at,
    set_pixel,
    fill_pixels,
)


# ============================================================================
# create_pixel_container
# ============================================================================


def test_create_sets_width_height():
    c = create_pixel_container(10, 20)
    assert c.width == 10
    assert c.height == 20


def test_create_data_length():
    c = create_pixel_container(4, 3)
    assert len(c.data) == 4 * 3 * 4  # 48


def test_create_zeroed():
    c = create_pixel_container(5, 5)
    assert all(b == 0 for b in c.data)


def test_create_zero_size():
    c = create_pixel_container(0, 0)
    assert len(c.data) == 0


def test_create_1x1():
    c = create_pixel_container(1, 1)
    assert len(c.data) == 4


# ============================================================================
# pixel_at
# ============================================================================


def test_pixel_at_fresh_is_zero():
    c = create_pixel_container(4, 4)
    assert pixel_at(c, 2, 2) == (0, 0, 0, 0)


def test_pixel_at_returns_set_value():
    c = create_pixel_container(4, 4)
    set_pixel(c, 1, 2, 200, 100, 50, 255)
    assert pixel_at(c, 1, 2) == (200, 100, 50, 255)


def test_pixel_at_oob_x():
    c = create_pixel_container(3, 3)
    assert pixel_at(c, 3, 0) == (0, 0, 0, 0)


def test_pixel_at_oob_y():
    c = create_pixel_container(3, 3)
    assert pixel_at(c, 0, 3) == (0, 0, 0, 0)


def test_pixel_at_negative():
    c = create_pixel_container(3, 3)
    assert pixel_at(c, -1, 0) == (0, 0, 0, 0)
    assert pixel_at(c, 0, -1) == (0, 0, 0, 0)


def test_pixel_at_row_major():
    # offset = (y * width + x) * 4; for (x=2, y=1, width=3): (1*3+2)*4 = 20
    c = create_pixel_container(3, 2)
    c.data[20] = 11
    c.data[21] = 22
    c.data[22] = 33
    c.data[23] = 44
    assert pixel_at(c, 2, 1) == (11, 22, 33, 44)


# ============================================================================
# set_pixel
# ============================================================================


def test_set_pixel_writes_rgba():
    c = create_pixel_container(4, 4)
    set_pixel(c, 0, 0, 10, 20, 30, 40)
    assert c.data[0] == 10
    assert c.data[1] == 20
    assert c.data[2] == 30
    assert c.data[3] == 40


def test_set_pixel_does_not_affect_neighbours():
    c = create_pixel_container(4, 4)
    set_pixel(c, 2, 1, 255, 0, 0, 255)
    assert pixel_at(c, 1, 1) == (0, 0, 0, 0)
    assert pixel_at(c, 3, 1) == (0, 0, 0, 0)


def test_set_pixel_oob_x_is_noop():
    c = create_pixel_container(2, 2)
    set_pixel(c, 99, 0, 1, 2, 3, 4)
    assert all(b == 0 for b in c.data)


def test_set_pixel_oob_y_is_noop():
    c = create_pixel_container(2, 2)
    set_pixel(c, 0, 99, 1, 2, 3, 4)
    assert all(b == 0 for b in c.data)


def test_set_pixel_round_trips():
    c = create_pixel_container(10, 10)
    set_pixel(c, 5, 7, 128, 64, 32, 200)
    assert pixel_at(c, 5, 7) == (128, 64, 32, 200)


# ============================================================================
# fill_pixels
# ============================================================================


def test_fill_sets_all_pixels():
    c = create_pixel_container(3, 3)
    fill_pixels(c, 100, 150, 200, 255)
    for y in range(3):
        for x in range(3):
            assert pixel_at(c, x, y) == (100, 150, 200, 255)


def test_fill_overwrites():
    c = create_pixel_container(2, 2)
    set_pixel(c, 0, 0, 255, 0, 0, 255)
    fill_pixels(c, 0, 0, 0, 0)
    assert pixel_at(c, 0, 0) == (0, 0, 0, 0)


def test_fill_zero_size_no_error():
    c = create_pixel_container(0, 0)
    fill_pixels(c, 1, 2, 3, 4)  # should not raise


# ============================================================================
# PixelContainer dataclass
# ============================================================================


def test_pixel_container_dataclass():
    c = PixelContainer(width=2, height=2, data=bytearray(16))
    assert c.width == 2
    assert c.height == 2
    assert len(c.data) == 16


def test_pixel_container_default_data():
    # When data is empty bytearray (default), __post_init__ allocates
    c = PixelContainer(width=3, height=2)
    assert len(c.data) == 24


# ============================================================================
# ImageCodec ABC
# ============================================================================


class _StubCodec(ImageCodec):
    @property
    def mime_type(self) -> str:
        return "image/stub"

    def encode(self, container: PixelContainer) -> bytes:
        return bytes([container.width, container.height])

    def decode(self, data: bytes) -> PixelContainer:
        return create_pixel_container(data[0], data[1])


def test_image_codec_stub():
    codec = _StubCodec()
    assert codec.mime_type == "image/stub"
    c = create_pixel_container(5, 3)
    enc = codec.encode(c)
    assert enc == bytes([5, 3])
    dec = codec.decode(enc)
    assert dec.width == 5
    assert dec.height == 3


def test_image_codec_is_abstract():
    with pytest.raises(TypeError):
        ImageCodec()  # type: ignore[abstract]
