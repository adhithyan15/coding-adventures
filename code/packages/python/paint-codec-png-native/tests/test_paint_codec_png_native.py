from paint_codec_png_native import available, encode
from pixel_container import PixelContainer


def test_available_when_extension_is_loaded() -> None:
    assert available()


def test_encode_png() -> None:
    pixels = PixelContainer(width=1, height=1, data=bytearray([0, 0, 0, 255]))
    png = encode(pixels)
    assert png[:8] == b"\x89PNG\r\n\x1a\n"
