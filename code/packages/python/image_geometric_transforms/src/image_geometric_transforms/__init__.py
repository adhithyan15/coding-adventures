"""
coding-adventures-image-geometric-transforms

IMG04 — Geometric transformations on PixelContainer.

A geometric transform repositions pixels in 2-D space.  Unlike point
operations (IMG03), geometric transforms change WHICH pixel appears at each
output location — potentially blending neighbours when the mapping is
non-integer.

## The inverse-warp model

There are two ways to implement a spatial transform:

    Forward warp: for each source pixel, compute where it lands in the output.
    Inverse warp: for each output pixel, compute where to fetch from the source.

Inverse warp is overwhelmingly preferred because it guarantees exactly one
write per output pixel — forward warp can leave holes or cause over-writes.
Every continuous transform here (scale, rotate, affine, perspective_warp) uses
inverse warp:

    for y' in 0..out_h:
        for x' in 0..out_w:
            (u, v) = inverse_transform(x', y')   # fractional source coord
            out[x', y'] = sample(src, u, v)

## Pixel-centre model

A pixel at integer coordinates (x, y) represents a 1×1 cell centred at that
position.  The *centre* of pixel (0, 0) is at (0.5, 0.5) in continuous space,
and a W-pixel-wide image spans [0, W] in continuous space.

When scaling, this shifts the mapping slightly.  Instead of  u = x' * (W/W'),
we use:
    u = (x' + 0.5) * (W / W') - 0.5

This ensures pixel centres map to pixel centres when the output size equals
the input size, and distributes the border pixels symmetrically.

## Colour-correct blending

Bilinear and bicubic sampling blend multiple source pixels.  Blending in sRGB
(the non-linear byte storage format) would under-weight dark colours because
sRGB is gamma-compressed.  We instead:

    1. Decode each source byte to linear light (via the sRGB transfer curve).
    2. Perform all interpolation arithmetic in linear light.
    3. Re-encode the result back to sRGB.

## Lossless transforms

Flip, 90°/180° rotation, crop, and pad only copy or rearrange whole pixels.
They never blend, so no sRGB decode/encode is needed — we copy bytes directly.
"""
from __future__ import annotations

import math
from enum import Enum

from pixel_container import PixelContainer, create_pixel_container, pixel_at, set_pixel

__all__ = [
    "Interpolation",
    "RotateBounds",
    "OutOfBounds",
    "flip_horizontal",
    "flip_vertical",
    "rotate_90_cw",
    "rotate_90_ccw",
    "rotate_180",
    "crop",
    "pad",
    "scale",
    "rotate",
    "affine",
    "perspective_warp",
]

VERSION = "0.1.0"

# ── sRGB ↔ linear LUT (built once at module load time) ────────────────────────
#
# The IEC 61966-2-1 sRGB standard defines a piecewise transfer function:
#
#   linear = C / 12.92                     when C <= 0.04045
#          = ((C + 0.055) / 1.055) ^ 2.4  otherwise
#
# where C = byte / 255.  The threshold 0.04045 corresponds to a linear
# value of ~0.003130803, making the curve continuous at that point.
#
# We precompute all 256 values into a list so that _decode() is O(1) and
# avoids re-evaluating the piecewise formula on every pixel sample.

_SRGB_TO_LINEAR: list[float] = [
    c / 12.92 if (c := i / 255.0) <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    for i in range(256)
]


def _decode(b: int) -> float:
    """sRGB u8 → linear f32.  Looks up the precomputed LUT."""
    return _SRGB_TO_LINEAR[b]


def _encode(v: float) -> int:
    """linear f32 → sRGB u8.

    Applies the inverse transfer function and clamps to [0, 255].
    The two branches mirror the IEC 61966-2-1 decode formula in reverse:

        encoded = 12.92 * v                     when v <= 0.0031308
                = 1.055 * v^(1/2.4) - 0.055    otherwise
    """
    c = 12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055
    return round(min(1.0, max(0.0, c)) * 255)


# ── Enum types ────────────────────────────────────────────────────────────────


class Interpolation(Enum):
    """Pixel-blending strategy for continuous (non-integer) source lookups.

    NEAREST   — Snap to the closest pixel.  Fast; looks blocky when scaled up.
    BILINEAR  — Weighted blend of the 2×2 surrounding pixels.  Smooth, slightly
                blurry; the standard choice for most use-cases.
    BICUBIC   — Catmull-Rom blend of the 4×4 surrounding pixels.  Sharper than
                bilinear with less ringing than Lanczos; best for downscaling.
    """
    NEAREST = "nearest"
    BILINEAR = "bilinear"
    BICUBIC = "bicubic"


class RotateBounds(Enum):
    """How to handle the output canvas size for arbitrary-angle rotation.

    FIT  — Expand the canvas so the rotated image fits entirely within it,
           with transparent corners filled with zeros (no clipping).
    CROP — Keep the original image dimensions; the rotated corners are clipped
           and filled with transparent black.
    """
    FIT = "fit"
    CROP = "crop"


class OutOfBounds(Enum):
    """Policy for sampling outside the source image boundary.

    ZERO       — Return (0, 0, 0, 0) transparent black for any OOB coordinate.
    REPLICATE  — Clamp to the nearest edge pixel (extend the border colour).
    REFLECT    — Mirror the image at its edges (tiles like a mirror).
    WRAP       — Tile the image periodically (modulo wrap).
    """
    ZERO = "zero"
    REPLICATE = "replicate"
    REFLECT = "reflect"
    WRAP = "wrap"


# ── Out-of-bounds coordinate resolution ───────────────────────────────────────
#
# Before reading a source pixel at integer (px, py) we must decide what to do
# when that coordinate is outside [0, max_).  The four policies produce
# different tiling/border behaviours.


def _resolve(x: int, max_: int, oob: OutOfBounds) -> int | None:
    """Map a possibly-OOB integer coordinate to a valid source index, or None.

    Args:
        x:    The integer coordinate (may be negative or >= max_).
        max_: The image dimension in this axis (width or height).
        oob:  The boundary policy.

    Returns:
        A valid index in [0, max_), or None when oob=ZERO and coordinate is OOB.

    ZERO:
        Any coordinate outside [0, max_) returns None, signalling the caller
        to use transparent black.

    REPLICATE:
        Clamp to the range: negative coords snap to 0, coords >= max_ snap
        to max_-1.  The border pixel is repeated infinitely outside the image.

    REFLECT:
        The image is mirrored at both edges.  Period = 2*max_.  Map x into
        [0, 2*max_) via modulo, then fold the upper half back:

            x in [0, max_)       → use x directly
            x in [max_, 2*max_)  → use 2*max_-1 - x

        Python's % operator always returns non-negative results, so a negative
        x is correctly handled without a separate branch.

    WRAP:
        Tile the image: x maps to x % max_, Python semantics guarantee the
        result is always in [0, max_).
    """
    if oob == OutOfBounds.ZERO:
        return None if (x < 0 or x >= max_) else x
    elif oob == OutOfBounds.REPLICATE:
        return max(0, min(max_ - 1, x))
    elif oob == OutOfBounds.REFLECT:
        period = 2 * max_
        x = x % period           # map into [0, 2*max_) — Python % is always >=0
        if x >= max_:
            x = period - 1 - x   # fold the upper half back
        return x
    else:  # WRAP
        return x % max_


def _fetch(img: PixelContainer, px: int, py: int, oob: OutOfBounds) -> tuple[int, int, int, int]:
    """Fetch a source pixel at integer (px, py) with OOB handling.

    Resolves both axes independently via _resolve, then delegates to pixel_at
    (which itself returns (0,0,0,0) for OOB — but _resolve with ZERO returns
    None so we short-circuit before calling pixel_at).
    """
    rx = _resolve(px, img.width, oob)
    ry = _resolve(py, img.height, oob)
    if rx is None or ry is None:
        return (0, 0, 0, 0)
    return pixel_at(img, rx, ry)


# ── Catmull-Rom cubic kernel ───────────────────────────────────────────────────
#
# Catmull-Rom is a cubic spline interpolation scheme that passes through its
# control points (unlike B-spline which only approximates them).  The weight
# function for distance d (in pixels) is:
#
#   For |d| < 1:   (1.5|d|^3 - 2.5|d|^2 + 1)
#   For 1<=|d|<2:  (-0.5|d|^3 + 2.5|d|^2 - 4|d| + 2)
#   Otherwise:     0
#
# This is the α=-0.5 variant of the Keys cubic family, commonly used in
# image processing.  It produces slightly sharper results than the α=-1
# Mitchell-Netravali filter.


def _catmull_rom(d: float) -> float:
    """Catmull-Rom cubic kernel weight at distance d.

    The kernel has support [-2, +2] — four neighbouring pixels contribute.
    Weights sum to 1.0 for any fractional position, ensuring no DC offset.
    """
    d = abs(d)
    if d < 1.0:
        return 1.5 * d**3 - 2.5 * d**2 + 1.0
    elif d < 2.0:
        return -0.5 * d**3 + 2.5 * d**2 - 4.0 * d + 2.0
    return 0.0


# ── Interpolation functions ───────────────────────────────────────────────────
#
# Each function takes a continuous (u, v) source coordinate and returns an
# (r, g, b, a) tuple.  The callers supply coordinates in source-image space,
# which may be fractional.


def _sample_nearest(
    img: PixelContainer,
    u: float,
    v: float,
    oob: OutOfBounds,
) -> tuple[int, int, int, int]:
    """Nearest-neighbour sampling: snap (u, v) to the closest integer pixel.

    round() selects the pixel whose centre is geometrically closest to the
    sub-pixel position.  No blending occurs, so no sRGB conversion is needed.
    """
    px = round(u)
    py = round(v)
    return _fetch(img, px, py, oob)


def _sample_bilinear(
    img: PixelContainer,
    u: float,
    v: float,
    oob: OutOfBounds,
) -> tuple[int, int, int, int]:
    """Bilinear sampling: weighted blend of the 2×2 surrounding pixels.

    We decompose the sub-pixel position into integer floor (x0, y0) and
    fractional remainder (fx, fy):

        x0 = floor(u);  fx = u - x0   (fx in [0, 1))
        y0 = floor(v);  fy = v - y0

    The four corners are weighted by the complementary fractions:

        weight of (x0,   y0  ) = (1-fx)*(1-fy)  — top-left
        weight of (x0+1, y0  ) = fx    *(1-fy)  — top-right
        weight of (x0,   y0+1) = (1-fx)*fy      — bottom-left
        weight of (x0+1, y0+1) = fx    *fy      — bottom-right

    Blending is performed in linear light to avoid gamma errors.
    Alpha is blended linearly (it is already scene-linear by convention).
    """
    x0 = math.floor(u)
    y0 = math.floor(v)
    fx = u - x0
    fy = v - y0

    def get_lin(px: int, py: int) -> tuple[float, float, float, float]:
        r, g, b, a = _fetch(img, px, py, oob)
        return _decode(r), _decode(g), _decode(b), a / 255.0

    # Four corners
    r00, g00, b00, a00 = get_lin(x0,     y0    )
    r10, g10, b10, a10 = get_lin(x0 + 1, y0    )
    r01, g01, b01, a01 = get_lin(x0,     y0 + 1)
    r11, g11, b11, a11 = get_lin(x0 + 1, y0 + 1)

    # Bilinear combination: lerp along x, then along y
    w00 = (1 - fx) * (1 - fy)
    w10 = fx       * (1 - fy)
    w01 = (1 - fx) * fy
    w11 = fx       * fy

    lr = r00 * w00 + r10 * w10 + r01 * w01 + r11 * w11
    lg = g00 * w00 + g10 * w10 + g01 * w01 + g11 * w11
    lb = b00 * w00 + b10 * w10 + b01 * w01 + b11 * w11
    la = a00 * w00 + a10 * w10 + a01 * w01 + a11 * w11

    return _encode(lr), _encode(lg), _encode(lb), round(min(1.0, max(0.0, la)) * 255)


def _sample_bicubic(
    img: PixelContainer,
    u: float,
    v: float,
    oob: OutOfBounds,
) -> tuple[int, int, int, int]:
    """Bicubic sampling: Catmull-Rom blend of the 4×4 surrounding pixels.

    We read the 4×4 neighbourhood centred on floor(u), floor(v).
    For each column j in [x0-1, x0+2] we accumulate a 1-D blend along y
    (using the Catmull-Rom weights at dy = v - j_y), then blend those four
    column results along x (using weights at dx = u - j_x).

    In practice: for each of the 4 rows we compute the horizontal blend,
    then do a vertical blend of those 4 row-blended values.

    Bicubic overshoots slightly (the Catmull-Rom kernel has negative lobes),
    so outputs must be clamped to [0, 1] in linear light before encoding.
    """
    x0 = math.floor(u)
    y0 = math.floor(v)

    # Accumulate per channel in linear light
    acc_r = acc_g = acc_b = acc_a = 0.0
    total_w = 0.0

    for j in range(-1, 3):    # rows: y0-1 .. y0+2
        wy = _catmull_rom(v - (y0 + j))
        for i in range(-1, 3):  # cols: x0-1 .. x0+2
            wx = _catmull_rom(u - (x0 + i))
            w = wx * wy
            r, g, b, a = _fetch(img, x0 + i, y0 + j, oob)
            acc_r += _decode(r) * w
            acc_g += _decode(g) * w
            acc_b += _decode(b) * w
            acc_a += (a / 255.0) * w
            total_w += w

    # Catmull-Rom weights sum to 1.0 analytically, but floating-point
    # accumulation may drift slightly.  Divide to normalise; guard zero.
    if total_w != 0.0:
        acc_r /= total_w
        acc_g /= total_w
        acc_b /= total_w
        acc_a /= total_w

    return (
        _encode(max(0.0, min(1.0, acc_r))),
        _encode(max(0.0, min(1.0, acc_g))),
        _encode(max(0.0, min(1.0, acc_b))),
        round(min(1.0, max(0.0, acc_a)) * 255),
    )


def _sample(
    img: PixelContainer,
    u: float,
    v: float,
    mode: Interpolation,
    oob: OutOfBounds,
) -> tuple[int, int, int, int]:
    """Dispatch to the appropriate sampling function."""
    if mode == Interpolation.NEAREST:
        return _sample_nearest(img, u, v, oob)
    elif mode == Interpolation.BILINEAR:
        return _sample_bilinear(img, u, v, oob)
    else:
        return _sample_bicubic(img, u, v, oob)


# ── Lossless integer transforms ───────────────────────────────────────────────
#
# These transforms only copy whole pixels without blending, so we can
# manipulate raw bytes directly.  No sRGB encode/decode is required.


def flip_horizontal(src: PixelContainer) -> PixelContainer:
    """Flip the image left-to-right (mirror around the vertical axis).

    For each row, pixel at column x maps to column (W-1-x) in the output.
    We swap bytes directly: each pixel is 4 bytes (RGBA), so the stride is 4.

    Applying flip_horizontal twice returns the original image exactly.
    """
    w, h = src.width, src.height
    out = create_pixel_container(w, h)
    for y in range(h):
        for x in range(w):
            # Source pixel offset in this row
            src_off = (y * w + x) * 4
            # Mirror destination: rightmost column gets leftmost pixel
            dst_off = (y * w + (w - 1 - x)) * 4
            out.data[dst_off:dst_off + 4] = src.data[src_off:src_off + 4]
    return out


def flip_vertical(src: PixelContainer) -> PixelContainer:
    """Flip the image top-to-bottom (mirror around the horizontal axis).

    Row y maps to row (H-1-y) in the output.  We copy entire rows at once,
    which is efficient since rows are contiguous in memory.

    Applying flip_vertical twice returns the original image exactly.
    """
    w, h = src.width, src.height
    out = create_pixel_container(w, h)
    row_bytes = w * 4
    for y in range(h):
        src_off = y * row_bytes
        dst_off = (h - 1 - y) * row_bytes
        out.data[dst_off:dst_off + row_bytes] = src.data[src_off:src_off + row_bytes]
    return out


def rotate_90_cw(src: PixelContainer) -> PixelContainer:
    """Rotate 90° clockwise.

    The output image has swapped dimensions: out_width = src.height,
    out_height = src.width.

    Mapping — given an output pixel (x_out, y_out), the source pixel is:

        src_x = y_out              (output row → source column)
        src_y = (H - 1) - x_out   (output column, counting from the right → source row)

    where H = src.height.

    Intuition: the first (left) column of the source becomes the bottom row of
    the output; the first (top) row of the source becomes the last (right)
    column of the output.  That is, the top-left corner ends up at the
    top-right of the output.
    """
    W, H = src.width, src.height
    out = create_pixel_container(H, W)   # swapped: out_width=H, out_height=W
    for y_out in range(W):               # output rows 0 .. W-1
        for x_out in range(H):           # output cols 0 .. H-1
            # Inverse mapping: which source pixel feeds this output pixel?
            src_x = y_out
            src_y = (H - 1) - x_out
            src_off = (src_y * W + src_x) * 4
            dst_off = (y_out * H + x_out) * 4
            out.data[dst_off:dst_off + 4] = src.data[src_off:src_off + 4]
    return out


def rotate_90_ccw(src: PixelContainer) -> PixelContainer:
    """Rotate 90° counter-clockwise.

    Mapping — given an output pixel (x_out, y_out), the source pixel is:

        src_x = (W - 1) - y_out   (output row, counting from the bottom → source column)
        src_y = x_out              (output column → source row)

    where W = src.width.

    Intuition: the first (top) row of the source becomes the first (left)
    column of the output; the top-left corner goes to the bottom-left of the
    output.  rotate_90_cw followed by rotate_90_ccw returns the original image.
    """
    W, H = src.width, src.height
    out = create_pixel_container(H, W)   # swapped: out_width=H, out_height=W
    for y_out in range(W):               # output rows 0 .. W-1
        for x_out in range(H):           # output cols 0 .. H-1
            src_x = (W - 1) - y_out
            src_y = x_out
            src_off = (src_y * W + src_x) * 4
            dst_off = (y_out * H + x_out) * 4
            out.data[dst_off:dst_off + 4] = src.data[src_off:src_off + 4]
    return out


def rotate_180(src: PixelContainer) -> PixelContainer:
    """Rotate 180° (equivalent to flip_horizontal followed by flip_vertical).

    Mapping:
        out[x'][y'] = src[W - 1 - x'][H - 1 - y']

    Dimensions are unchanged.  Applying twice returns the original image.
    """
    W, H = src.width, src.height
    out = create_pixel_container(W, H)
    for y in range(H):
        for x in range(W):
            src_off = ((H - 1 - y) * W + (W - 1 - x)) * 4
            dst_off = (y * W + x) * 4
            out.data[dst_off:dst_off + 4] = src.data[src_off:src_off + 4]
    return out


def crop(src: PixelContainer, x0: int, y0: int, w: int, h: int) -> PixelContainer:
    """Extract a rectangular sub-region from the image.

    Args:
        src:  Source image.
        x0:   Left edge column (inclusive).
        y0:   Top edge row (inclusive).
        w:    Width of the crop region.
        h:    Height of the crop region.

    Pixels outside the source boundary are filled with transparent black
    (the default fill of create_pixel_container).  This avoids raising on
    partial out-of-bounds crops.

    The output has dimensions (w, h).
    """
    out = create_pixel_container(w, h)
    for dy in range(h):
        for dx in range(w):
            px = x0 + dx
            py = y0 + dy
            # pixel_at returns (0,0,0,0) for OOB — exactly what we want
            r, g, b, a = pixel_at(src, px, py)
            set_pixel(out, dx, dy, r, g, b, a)
    return out


def pad(
    src: PixelContainer,
    top: int,
    right: int,
    bottom: int,
    left: int,
    fill: tuple[int, int, int, int] = (0, 0, 0, 0),
) -> PixelContainer:
    """Add a border of `fill` colour around the image.

    Args:
        src:    Source image.
        top:    Pixels to add at the top.
        right:  Pixels to add at the right.
        bottom: Pixels to add at the bottom.
        left:   Pixels to add at the left.
        fill:   RGBA colour for the new border pixels (default: transparent black).

    Output dimensions:
        width  = src.width  + left + right
        height = src.height + top  + bottom

    The source image is placed at offset (left, top) in the output.
    """
    out_w = src.width  + left + right
    out_h = src.height + top  + bottom
    out = create_pixel_container(out_w, out_h)
    fr, fg, fb, fa = fill

    # Flood fill with border colour first (row by row for efficiency)
    for y in range(out_h):
        for x in range(out_w):
            # Is this coordinate inside the original image area?
            sx = x - left
            sy = y - top
            if 0 <= sx < src.width and 0 <= sy < src.height:
                r, g, b, a = pixel_at(src, sx, sy)
                set_pixel(out, x, y, r, g, b, a)
            else:
                set_pixel(out, x, y, fr, fg, fb, fa)
    return out


# ── Continuous transforms ─────────────────────────────────────────────────────
#
# These transforms map each output pixel to a (possibly fractional) source
# coordinate, then use an interpolation function to blend source pixels.


def scale(
    src: PixelContainer,
    out_w: int,
    out_h: int,
    mode: Interpolation = Interpolation.BILINEAR,
) -> PixelContainer:
    """Resize the image to (out_w, out_h).

    Uses the pixel-centre model to compute source coordinates:

        sx = out_w / src.width          (horizontal scale factor)
        sy = out_h / src.height
        u  = (x' + 0.5) / sx - 0.5     (source u for output column x')
        v  = (y' + 0.5) / sy - 0.5

    This formula ensures:
        - Pixel centres map to pixel centres at 1:1 scale.
        - The extreme output pixels map to the extreme source pixels, not beyond.
        - Both edges are treated symmetrically (no half-pixel bias).

    Out-of-bounds behaviour: REPLICATE (clamp to nearest edge) so that the
    border colour extends rather than leaving transparent strips.
    """
    oob = OutOfBounds.REPLICATE
    # Precompute the reciprocal scale factors once outside the loop
    inv_sx = src.width  / out_w
    inv_sy = src.height / out_h

    out = create_pixel_container(out_w, out_h)
    for y_out in range(out_h):
        for x_out in range(out_w):
            # Map output pixel centre to source space
            u = (x_out + 0.5) * inv_sx - 0.5
            v = (y_out + 0.5) * inv_sy - 0.5
            r, g, b, a = _sample(src, u, v, mode, oob)
            set_pixel(out, x_out, y_out, r, g, b, a)
    return out


def rotate(
    src: PixelContainer,
    radians: float,
    mode: Interpolation = Interpolation.BILINEAR,
    bounds: RotateBounds = RotateBounds.FIT,
) -> PixelContainer:
    """Rotate the image by `radians` counter-clockwise around its centre.

    Inverse warp: for each output pixel (x', y'), we rotate backwards by
    -radians to find the source coordinate (u, v):

        u = cx_in + cos_t*(x' - cx_out) + sin_t*(y' - cy_out)
        v = cy_in - sin_t*(x' - cx_out) + cos_t*(y' - cy_out)

    where (cx_in, cy_in) and (cx_out, cy_out) are the image centres.

    Note: the forward rotation matrix R(θ) maps (x, y) → (x', y') via:
        x' =  cos θ * x - sin θ * y
        y' =  sin θ * x + cos θ * y

    The inverse rotation R(-θ) maps output back to source:
        u = cos θ * (x' - cx_out) + sin θ * (y' - cy_out) + cx_in
        v = -sin θ * (x' - cx_out) + cos θ * (y' - cy_out) + cy_in

    RotateBounds.FIT:
        The bounding box of the rotated image is:
            out_w = ceil(W * |cos θ| + H * |sin θ|)
            out_h = ceil(W * |sin θ| + H * |cos θ|)

    RotateBounds.CROP:
        The canvas stays the same size; corners are clipped.

    OOB is ZERO — trimmed corners become transparent black.
    """
    W, H = src.width, src.height
    cos_t = math.cos(radians)
    sin_t = math.sin(radians)
    abs_cos = abs(cos_t)
    abs_sin = abs(sin_t)

    if bounds == RotateBounds.FIT:
        out_w = math.ceil(W * abs_cos + H * abs_sin)
        out_h = math.ceil(W * abs_sin + H * abs_cos)
    else:  # CROP
        out_w, out_h = W, H

    cx_in  = W / 2.0
    cy_in  = H / 2.0
    cx_out = out_w / 2.0
    cy_out = out_h / 2.0

    oob = OutOfBounds.ZERO
    out = create_pixel_container(out_w, out_h)

    for y_out in range(out_h):
        for x_out in range(out_w):
            dx = x_out - cx_out
            dy = y_out - cy_out
            # Inverse rotation: rotate (dx, dy) by -radians
            u = cx_in + cos_t * dx + sin_t * dy
            v = cy_in - sin_t * dx + cos_t * dy
            r, g, b, a = _sample(src, u, v, mode, oob)
            set_pixel(out, x_out, y_out, r, g, b, a)
    return out


def affine(
    src: PixelContainer,
    matrix: list[list[float]],
    out_w: int,
    out_h: int,
    mode: Interpolation = Interpolation.BILINEAR,
    oob: OutOfBounds = OutOfBounds.REPLICATE,
) -> PixelContainer:
    """Apply a 2×3 affine inverse-warp matrix to the image.

    The matrix encodes the *inverse* transform: given an output pixel (x', y'),
    it computes the source coordinate (u, v):

        u = m[0][0]*x' + m[0][1]*y' + m[0][2]
        v = m[1][0]*x' + m[1][1]*y' + m[1][2]

    This is the standard form used in OpenCV (cv2.warpAffine with WARP_INVERSE_MAP).

    The 2×3 matrix packs a 2×2 linear part and a 2×1 translation:

        [ a  b  tx ]     u = a*x' + b*y' + tx
        [ c  d  ty ]     v = c*x' + d*y' + ty

    Examples:
        Identity:     [[1, 0, 0], [0, 1, 0]]
        Scale ×2:     [[0.5, 0, 0], [0, 0.5, 0]]
        Rotate 45°:   precomputed cos/sin matrix + translation to keep in frame

    Args:
        src:    Source image.
        matrix: 2×3 inverse warp matrix (list of two 3-element rows).
        out_w:  Output width.
        out_h:  Output height.
        mode:   Interpolation mode.
        oob:    Out-of-bounds policy.
    """
    m = matrix
    out = create_pixel_container(out_w, out_h)
    for y_out in range(out_h):
        for x_out in range(out_w):
            u = m[0][0] * x_out + m[0][1] * y_out + m[0][2]
            v = m[1][0] * x_out + m[1][1] * y_out + m[1][2]
            r, g, b, a = _sample(src, u, v, mode, oob)
            set_pixel(out, x_out, y_out, r, g, b, a)
    return out


def perspective_warp(
    src: PixelContainer,
    h: list[list[float]],
    out_w: int,
    out_h: int,
    mode: Interpolation = Interpolation.BILINEAR,
    oob: OutOfBounds = OutOfBounds.REPLICATE,
) -> PixelContainer:
    """Apply a 3×3 homographic (perspective) inverse-warp to the image.

    A perspective transform models the projection of a planar surface onto a
    camera sensor.  Unlike affine transforms, parallel lines may converge
    (perspective foreshortening).

    The homography H is a 3×3 matrix operating in *homogeneous* coordinates.
    For each output pixel (x', y'), we compute:

        [uh]   [h[0][0]  h[0][1]  h[0][2]] [x']
        [vh] = [h[1][0]  h[1][1]  h[1][2]] [y']
        [w ]   [h[2][0]  h[2][1]  h[2][2]] [1 ]

        u = uh / w
        v = vh / w

    The homogeneous divide (dividing by w) is what produces the perspective
    effect — the scale of features shrinks with depth.

    When H is the 3×3 identity matrix, w = 1 for all pixels, so u = x', v = y':
    the transform is an identity.

    Args:
        src:    Source image.
        h:      3×3 inverse perspective homography matrix.
        out_w:  Output width.
        out_h:  Output height.
        mode:   Interpolation mode.
        oob:    Out-of-bounds policy.
    """
    out = create_pixel_container(out_w, out_h)
    for y_out in range(out_h):
        for x_out in range(out_w):
            # Homogeneous coordinates: multiply H by [x', y', 1]^T
            uh = h[0][0] * x_out + h[0][1] * y_out + h[0][2]
            vh = h[1][0] * x_out + h[1][1] * y_out + h[1][2]
            w_ = h[2][0] * x_out + h[2][1] * y_out + h[2][2]
            # Guard against degenerate projection (w very close to zero)
            if abs(w_) < 1e-10:
                set_pixel(out, x_out, y_out, 0, 0, 0, 0)
                continue
            u = uh / w_
            v = vh / w_
            r, g, b, a = _sample(src, u, v, mode, oob)
            set_pixel(out, x_out, y_out, r, g, b, a)
    return out
