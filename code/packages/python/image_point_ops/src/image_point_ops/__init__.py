"""
coding-adventures-image-point-ops

IMG03 — Per-pixel point operations on PixelContainer.

A point operation transforms each pixel independently using only that
pixel's value — no neighbourhood, no frequency domain, no geometry.

## Two domains

u8-domain operations (invert, threshold, posterize, channel ops, brightness)
work directly on the 8-bit sRGB bytes.  Correct without colour-space
conversion because they are monotone remappings that do not mix values.

Linear-light operations (contrast, gamma, exposure, greyscale, sepia,
colour_matrix, saturate, hue_rotate) decode to linear f32 first:

    c = byte / 255
    linear = c / 12.92           if c <= 0.04045
           = ((c + 0.055)/1.055)^2.4  otherwise

Then re-encode after the operation:

    encoded = linear * 12.92                        if linear <= 0.0031308
            = 1.055 * linear^(1/2.4) − 0.055       otherwise
    byte = round(clamp(encoded, 0, 1) * 255)
"""
from __future__ import annotations

import math
from enum import Enum
from typing import Callable

from pixel_container import PixelContainer, create_pixel_container, pixel_at, set_pixel

__all__ = [
    "invert",
    "threshold",
    "threshold_luminance",
    "posterize",
    "swap_rgb_bgr",
    "extract_channel",
    "brightness",
    "contrast",
    "gamma",
    "exposure",
    "GreyscaleMethod",
    "greyscale",
    "sepia",
    "colour_matrix",
    "saturate",
    "hue_rotate",
    "srgb_to_linear_image",
    "linear_to_srgb_image",
    "apply_lut1d_u8",
    "build_lut1d_u8",
    "build_gamma_lut",
]

VERSION = "0.1.0"

# ── sRGB ↔ linear LUT (built once at import time) ─────────────────────────

_SRGB_TO_LINEAR: list[float] = []
for _i in range(256):
    _c = _i / 255.0
    _SRGB_TO_LINEAR.append(_c / 12.92 if _c <= 0.04045 else ((_c + 0.055) / 1.055) ** 2.4)


def _decode(byte: int) -> float:
    """sRGB u8 → linear f32."""
    return _SRGB_TO_LINEAR[byte]


def _encode(linear: float) -> int:
    """linear f32 → sRGB u8 (clamped to [0, 255])."""
    if linear <= 0.0031308:
        c = linear * 12.92
    else:
        c = 1.055 * linear ** (1.0 / 2.4) - 0.055
    return round(min(1.0, max(0.0, c)) * 255)


# ── Iteration helper ───────────────────────────────────────────────────────

def _map_pixels(
    src: PixelContainer,
    fn: Callable[[int, int, int, int], tuple[int, int, int, int]],
) -> PixelContainer:
    out = create_pixel_container(src.width, src.height)
    for y in range(src.height):
        for x in range(src.width):
            r, g, b, a = pixel_at(src, x, y)
            nr, ng, nb, na = fn(r, g, b, a)
            set_pixel(out, x, y, nr, ng, nb, na)
    return out


# ── u8-domain operations ───────────────────────────────────────────────────

def invert(src: PixelContainer) -> PixelContainer:
    """Flip each RGB channel (255 − v).  Alpha is preserved.

    Applying invert twice returns the original image exactly because
    (255 − (255 − v)) == v for all integers in [0, 255].
    """
    return _map_pixels(src, lambda r, g, b, a: (255 - r, 255 - g, 255 - b, a))


def threshold(src: PixelContainer, value: int) -> PixelContainer:
    """Binarise on average luminance.  (r+g+b)/3 >= value → white, else black.
    Alpha is preserved.  Use threshold_luminance for perceptual accuracy.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        luma = (r + g + b) / 3
        v = 255 if luma >= value else 0
        return v, v, v, a
    return _map_pixels(src, _fn)


def threshold_luminance(src: PixelContainer, value: int) -> PixelContainer:
    """Binarise on Rec. 709 luma: Y = 0.2126 R + 0.7152 G + 0.0722 B.
    More perceptually accurate than simple average.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        luma = 0.2126 * r + 0.7152 * g + 0.0722 * b
        v = 255 if luma >= value else 0
        return v, v, v, a
    return _map_pixels(src, _fn)


def posterize(src: PixelContainer, levels: int) -> PixelContainer:
    """Reduce each channel to `levels` equally-spaced steps.

    levels = 2  →  poster / high-contrast look.
    levels = 256 →  identity.
    """
    step = 255.0 / (levels - 1)

    def _q(v: int) -> int:
        return round(round(v / step) * step)

    return _map_pixels(src, lambda r, g, b, a: (_q(r), _q(g), _q(b), a))


def swap_rgb_bgr(src: PixelContainer) -> PixelContainer:
    """Swap R and B channels (RGB ↔ BGR).

    Useful when an upstream codec emits BGR byte order.
    """
    return _map_pixels(src, lambda r, g, b, a: (b, g, r, a))


def extract_channel(src: PixelContainer, channel: int) -> PixelContainer:
    """Keep only the nominated channel (0=R, 1=G, 2=B, 3=A), zero the rest.
    Alpha is always preserved.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        vals = (r, g, b, a)
        v = vals[channel]
        if channel == 0:
            return v, 0, 0, a
        if channel == 1:
            return 0, v, 0, a
        if channel == 2:
            return 0, 0, v, a
        return r, g, b, v
    return _map_pixels(src, _fn)


def brightness(src: PixelContainer, offset: int) -> PixelContainer:
    """Add a signed offset to each RGB channel, clamped to [0, 255].
    Alpha is preserved.

    This is a u8-domain operation — linear shift in sRGB, not linear light.
    Fast and lossless on integer data.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        clamp = lambda v: min(255, max(0, v + offset))  # noqa: E731
        return clamp(r), clamp(g), clamp(b), a
    return _map_pixels(src, _fn)


# ── Linear-light operations ────────────────────────────────────────────────

def contrast(src: PixelContainer, factor: float) -> PixelContainer:
    """Scale each linear channel around mid-grey (0.5 linear).

    factor = 1.0 → identity; < 1.0 → lower contrast; > 1.0 → higher.
    Formula: linear_out = 0.5 + factor * (linear_in − 0.5)
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        return (
            _encode(0.5 + factor * (_decode(r) - 0.5)),
            _encode(0.5 + factor * (_decode(g) - 0.5)),
            _encode(0.5 + factor * (_decode(b) - 0.5)),
            a,
        )
    return _map_pixels(src, _fn)


def gamma(src: PixelContainer, g: float) -> PixelContainer:
    """Apply a power-law γ to each linear channel.

    γ < 1 → brightens; γ > 1 → darkens; γ = 1 → identity.
    Formula: linear_out = linear_in ^ γ
    """
    def _fn(r: int, gv: int, b: int, a: int) -> tuple[int, int, int, int]:
        return (
            _encode(_decode(r) ** g),
            _encode(_decode(gv) ** g),
            _encode(_decode(b) ** g),
            a,
        )
    return _map_pixels(src, _fn)


def exposure(src: PixelContainer, stops: float) -> PixelContainer:
    """Multiply linear luminance by 2^stops.

    +1 stop → double the light; −1 stop → halve.
    """
    factor = 2.0 ** stops
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        return (
            _encode(_decode(r) * factor),
            _encode(_decode(g) * factor),
            _encode(_decode(b) * factor),
            a,
        )
    return _map_pixels(src, _fn)


class GreyscaleMethod(Enum):
    """Luminance weighting scheme for greyscale conversion."""
    REC709 = "rec709"   # 0.2126 R + 0.7152 G + 0.0722 B  (perceptually correct)
    BT601 = "bt601"     # 0.2989 R + 0.5870 G + 0.1140 B  (legacy SD-TV)
    AVERAGE = "average" # (R + G + B) / 3                   (equal weights, fast)


def greyscale(src: PixelContainer, method: GreyscaleMethod = GreyscaleMethod.REC709) -> PixelContainer:
    """Convert to luminance in linear light, re-encode to sRGB.

    Computed in linear light so that the result is physically correct.
    Averaging in sRGB space would produce a slightly darker result.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        lr, lg, lb = _decode(r), _decode(g), _decode(b)
        if method == GreyscaleMethod.REC709:
            y = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb
        elif method == GreyscaleMethod.BT601:
            y = 0.2989 * lr + 0.5870 * lg + 0.1140 * lb
        else:
            y = (lr + lg + lb) / 3.0
        out = _encode(y)
        return out, out, out, a
    return _map_pixels(src, _fn)


def sepia(src: PixelContainer) -> PixelContainer:
    """Apply a classic warm sepia tone matrix in linear light.

    The sepia matrix desaturates and shifts towards red-orange — the
    photographic darkroom effect from iron-gall ink development.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        lr, lg, lb = _decode(r), _decode(g), _decode(b)
        return (
            _encode(0.393 * lr + 0.769 * lg + 0.189 * lb),
            _encode(0.349 * lr + 0.686 * lg + 0.168 * lb),
            _encode(0.272 * lr + 0.534 * lg + 0.131 * lb),
            a,
        )
    return _map_pixels(src, _fn)


def colour_matrix(
    src: PixelContainer,
    matrix: list[list[float]],
) -> PixelContainer:
    """Multiply linear [R, G, B] by a 3×3 matrix.

    matrix is row-major: [[r_row], [g_row], [b_row]].
    Identity: [[1,0,0],[0,1,0],[0,0,1]].
    """
    m = matrix
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        lr, lg, lb = _decode(r), _decode(g), _decode(b)
        return (
            _encode(m[0][0] * lr + m[0][1] * lg + m[0][2] * lb),
            _encode(m[1][0] * lr + m[1][1] * lg + m[1][2] * lb),
            _encode(m[2][0] * lr + m[2][1] * lg + m[2][2] * lb),
            a,
        )
    return _map_pixels(src, _fn)


def saturate(src: PixelContainer, factor: float) -> PixelContainer:
    """Scale saturation in linear RGB.

    factor = 0 → greyscale; 1 → identity; > 1 → hypersaturated.
    Uses Rec. 709 luminance to derive the achromatic grey value.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        lr, lg, lb = _decode(r), _decode(g), _decode(b)
        grey = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb
        return (
            _encode(grey + factor * (lr - grey)),
            _encode(grey + factor * (lg - grey)),
            _encode(grey + factor * (lb - grey)),
            a,
        )
    return _map_pixels(src, _fn)


# ── HSV helpers ────────────────────────────────────────────────────────────

def _rgb_to_hsv(r: float, g: float, b: float) -> tuple[float, float, float]:
    mx = max(r, g, b)
    mn = min(r, g, b)
    delta = mx - mn
    v = mx
    s = 0.0 if mx == 0.0 else delta / mx
    h = 0.0
    if delta != 0.0:
        if mx == r:
            h = ((g - b) / delta) % 6
        elif mx == g:
            h = (b - r) / delta + 2
        else:
            h = (r - g) / delta + 4
        h = (h * 60 + 360) % 360
    return h, s, v


def _hsv_to_rgb(h: float, s: float, v: float) -> tuple[float, float, float]:
    c = v * s
    x = c * (1 - abs((h / 60) % 2 - 1))
    m = v - c
    if h < 60:
        r, g, b = c, x, 0.0
    elif h < 120:
        r, g, b = x, c, 0.0
    elif h < 180:
        r, g, b = 0.0, c, x
    elif h < 240:
        r, g, b = 0.0, x, c
    elif h < 300:
        r, g, b = x, 0.0, c
    else:
        r, g, b = c, 0.0, x
    return r + m, g + m, b + m


def hue_rotate(src: PixelContainer, degrees: float) -> PixelContainer:
    """Rotate the hue of each pixel by `degrees` (0–360).

    Performed in linear-light HSV space.  360° is an identity.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        h, s, v = _rgb_to_hsv(_decode(r), _decode(g), _decode(b))
        nr, ng, nb = _hsv_to_rgb((h + degrees + 360) % 360, s, v)
        return _encode(nr), _encode(ng), _encode(nb), a
    return _map_pixels(src, _fn)


# ── Colorspace utilities ───────────────────────────────────────────────────

def srgb_to_linear_image(src: PixelContainer) -> PixelContainer:
    """Convert sRGB → linear by storing linear * 255 in each byte.

    Returns a PixelContainer whose bytes represent linear light values
    directly (not gamma-encoded).  Useful for arithmetic-on-bytes pipelines.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        return (
            round(_decode(r) * 255),
            round(_decode(g) * 255),
            round(_decode(b) * 255),
            a,
        )
    return _map_pixels(src, _fn)


def linear_to_srgb_image(src: PixelContainer) -> PixelContainer:
    """Convert linear → sRGB (inverse of srgb_to_linear_image)."""
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        return _encode(r / 255), _encode(g / 255), _encode(b / 255), a
    return _map_pixels(src, _fn)


# ── 1D LUT operations ──────────────────────────────────────────────────────

def apply_lut1d_u8(
    src: PixelContainer,
    lut_r: bytes | bytearray,
    lut_g: bytes | bytearray,
    lut_b: bytes | bytearray,
) -> PixelContainer:
    """Apply three 256-entry u8→u8 LUTs (one per channel) to the image.

    Alpha is always preserved.  Three separate LUTs allow different curves
    per channel (e.g. split-tone colour grading).

    A LUT reduces any per-pixel transform to a single array index, which
    is faster than recomputing the transform function per pixel.
    """
    def _fn(r: int, g: int, b: int, a: int) -> tuple[int, int, int, int]:
        return lut_r[r], lut_g[g], lut_b[b], a
    return _map_pixels(src, _fn)


def build_lut1d_u8(fn: Callable[[float], float]) -> bytearray:
    """Build a 256-entry LUT from a linear-light mapping function f: [0,1]→[0,1].

    Each input byte i is decoded to linear, fn is applied, then re-encoded.
    Lets you compile any linear-light curve into a fast u8 lookup.
    """
    lut = bytearray(256)
    for i in range(256):
        lut[i] = _encode(fn(_decode(i)))
    return lut


def build_gamma_lut(g: float) -> bytearray:
    """Build a gamma LUT: each byte is decoded, raised to γ, then re-encoded.

    Equivalent to build_lut1d_u8(lambda v: v ** gamma).
    γ < 1 → brightens; γ > 1 → darkens; γ = 1 → identity.
    """
    return build_lut1d_u8(lambda v: v ** g)
