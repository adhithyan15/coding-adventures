"""Python wrapper for the native PNG codec bridge."""

from __future__ import annotations

from pixel_container import PixelContainer

try:
    from paint_codec_png_native.paint_codec_png_native import (  # type: ignore[import]
        encode_rgba8_native as _encode_rgba8_native,
    )
except ImportError:  # pragma: no cover - exercised when the extension is absent
    _encode_rgba8_native = None

__all__ = ["PaintCodecPngNativeError", "available", "encode"]


class PaintCodecPngNativeError(RuntimeError):
    """Raised when the native PNG codec is unavailable."""


def available() -> bool:
    return _encode_rgba8_native is not None


def encode(pixels: PixelContainer) -> bytes:
    if _encode_rgba8_native is None:
        raise PaintCodecPngNativeError("paint_codec_png_native extension is not available")
    return _encode_rgba8_native(pixels.width, pixels.height, bytes(pixels.data))
