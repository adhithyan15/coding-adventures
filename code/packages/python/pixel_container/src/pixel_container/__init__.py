"""
coding-adventures-pixel-container

IC00: Universal RGBA8 pixel buffer and image codec interface.

This is the zero-dependency foundation for the IC image codec series.
Every image codec (BMP, PPM, QOI, ...) depends only on this package.

## Layout

Pixels are stored row-major, top-left origin, RGBA interleaved:

    offset = (y * width + x) * 4
    data[offset + 0] = R
    data[offset + 1] = G
    data[offset + 2] = B
    data[offset + 3] = A

Channel count and bit depth are fixed: 4 channels, 8 bits each (RGBA8).

## Why fixed RGBA8?

All three codecs in this series (BMP, PPM, QOI) operate on RGBA8 pixels.
A fixed format keeps every codec simple — no conditional logic for RGB vs
RGBA, no 16-bit paths.
"""
from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field

__all__ = [
    "PixelContainer",
    "ImageCodec",
    "create_pixel_container",
    "pixel_at",
    "set_pixel",
    "fill_pixels",
]

VERSION = "0.1.0"


# ============================================================================
# PixelContainer — the one data type
# ============================================================================


@dataclass
class PixelContainer:
    """A fixed-format RGBA8 pixel buffer.

    ``data`` is a flat ``bytearray`` of ``width × height × 4`` bytes.
    Pixels are stored row-major, top-left origin:

        offset = (y * width + x) * 4
        data[offset + 0] = R
        data[offset + 1] = G
        data[offset + 2] = B
        data[offset + 3] = A

    Example — 2×1 image, red then blue::

        c = create_pixel_container(2, 1)
        set_pixel(c, 0, 0, 255, 0, 0, 255)   # red at (0, 0)
        set_pixel(c, 1, 0, 0, 0, 255, 255)   # blue at (1, 0)
        pixel_at(c, 0, 0)   # → (255, 0, 0, 255)
    """

    width: int
    height: int
    data: bytearray = field(default_factory=bytearray)

    def __post_init__(self) -> None:
        # Allow callers to pass an empty bytearray and rely on default_factory,
        # or pass pre-built data directly (e.g. from a decoder).
        if not self.data:
            self.data = bytearray(self.width * self.height * 4)


# ============================================================================
# ImageCodec — encode/decode contract
# ============================================================================


class ImageCodec(ABC):
    """Abstract base class for image codecs.

    Subclasses implement :meth:`encode` and :meth:`decode` to convert
    between a :class:`PixelContainer` and raw file bytes.

    Canonical encode pipeline::

        c = create_pixel_container(320, 240)
        fill_pixels(c, 128, 0, 200, 255)     # purple
        raw = codec.encode(c)                 # → bytes
        Path("out.bmp").write_bytes(raw)

    Canonical decode pipeline::

        raw = Path("photo.bmp").read_bytes()
        c = codec.decode(raw)
        r, g, b, a = pixel_at(c, 10, 20)
    """

    @property
    @abstractmethod
    def mime_type(self) -> str:
        """MIME type string, e.g. ``"image/bmp"``."""

    @abstractmethod
    def encode(self, container: PixelContainer) -> bytes:
        """Encode a PixelContainer to raw file bytes."""

    @abstractmethod
    def decode(self, data: bytes) -> PixelContainer:
        """Decode raw file bytes to a PixelContainer."""


# ============================================================================
# create_pixel_container — factory
# ============================================================================


def create_pixel_container(width: int, height: int) -> PixelContainer:
    """Create a new pixel container filled with transparent black (R=G=B=A=0).

    ``width`` and ``height`` must be non-negative integers.
    A 0×0 container is valid and has an empty data buffer.

    Example::

        c = create_pixel_container(320, 240)
        assert len(c.data) == 320 * 240 * 4  # 307200
    """
    return PixelContainer(width=width, height=height, data=bytearray(width * height * 4))


# ============================================================================
# pixel_at — read one pixel
# ============================================================================


def pixel_at(c: PixelContainer, x: int, y: int) -> tuple[int, int, int, int]:
    """Return the RGBA components of the pixel at column ``x``, row ``y``.

    Returns ``(0, 0, 0, 0)`` for out-of-bounds coordinates so callers do not
    need to guard every border pixel read.

    Example::

        c = create_pixel_container(4, 4)
        set_pixel(c, 1, 2, 200, 100, 50, 255)
        assert pixel_at(c, 1, 2) == (200, 100, 50, 255)
        assert pixel_at(c, 99, 0) == (0, 0, 0, 0)   # out of bounds
    """
    if x < 0 or x >= c.width or y < 0 or y >= c.height:
        return (0, 0, 0, 0)
    i = (y * c.width + x) * 4
    return (c.data[i], c.data[i + 1], c.data[i + 2], c.data[i + 3])


# ============================================================================
# set_pixel — write one pixel
# ============================================================================


def set_pixel(c: PixelContainer, x: int, y: int, r: int, g: int, b: int, a: int) -> None:
    """Write the RGBA components of the pixel at column ``x``, row ``y``.

    No-op for out-of-bounds coordinates.

    Example::

        set_pixel(c, 2, 3, 255, 128, 0, 255)  # orange at (2, 3)
    """
    if x < 0 or x >= c.width or y < 0 or y >= c.height:
        return
    i = (y * c.width + x) * 4
    c.data[i]     = r & 0xFF
    c.data[i + 1] = g & 0xFF
    c.data[i + 2] = b & 0xFF
    c.data[i + 3] = a & 0xFF


# ============================================================================
# fill_pixels — flood fill
# ============================================================================


def fill_pixels(c: PixelContainer, r: int, g: int, b: int, a: int) -> None:
    """Set every pixel in the container to the given RGBA colour.

    Useful for clearing a canvas before drawing::

        fill_pixels(c, 255, 255, 255, 255)  # solid white
        fill_pixels(c, 0, 0, 0, 0)          # transparent black (clear)
    """
    rb = r & 0xFF
    gb = g & 0xFF
    bb = b & 0xFF
    ab = a & 0xFF
    for i in range(0, len(c.data), 4):
        c.data[i]     = rb
        c.data[i + 1] = gb
        c.data[i + 2] = bb
        c.data[i + 3] = ab
